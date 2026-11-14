//! A fake fleet of servers, producing plausible-looking metrics and logs.
//!
//! Nothing here draws anything: it is just the data source for the dashboard. The only
//! cursive type used is `Series`, a shared handle on a sequence of samples. The views hold
//! clones of the same handles, so they read the history without going through the `Sim`.
//! The other values (services, regions, health indicators) are plain fields: the views read
//! them through the shared `Sim` with closures.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cursive::views::Series;

/// State shared between the simulation thread and the views drawing it.
pub type Shared = Arc<Mutex<Sim>>;

/// How many samples each series remembers.
const HISTORY: usize = 400;
/// How many log lines we keep around.
const MAX_LOGS: usize = 300;
/// Simulated time between two steps, in milliseconds.
const STEP_MS: u64 = 250;

/// Tiny xorshift generator: enough for a demo, and reproducible.
pub struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Returns a value in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Returns a value in `[-1, 1)`.
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
    Deploy,
}

pub struct LogLine {
    pub time: String,
    pub level: Level,
    pub source: &'static str,
    pub message: String,
}

pub struct Service {
    pub name: &'static str,
    pub region: &'static str,
    /// Load, between 0 and 1.
    pub load: f32,
    pub replicas: usize,
    phase: f32,
}

/// A health indicator, between 0 and 1.
pub struct Indicator {
    pub name: &'static str,
    /// Current value.
    pub value: f32,
    pub higher_is_better: bool,
}

pub struct Node {
    pub name: String,
    pub heat: Series,
    phase: f32,
}

pub struct Region {
    pub name: &'static str,
    pub latency: f32,
    base: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Cpu,
    Memory,
    Requests,
    Errors,
}

pub struct Sim {
    rng: Rng,
    pub tick: u64,
    pub paused: bool,
    /// Between 0 (all quiet) and 1 (everything is on fire).
    pub incident: f32,
    wander: f32,
    /// Share of the error budget left, between 0 and 1.
    pub budget: f32,

    pub cpu: Series,
    pub memory: Series,
    pub requests: Series,
    pub errors: Series,
    pub p50: Series,
    pub p99: Series,
    pub gauges: Vec<Indicator>,

    pub services: Vec<Service>,
    pub nodes: Vec<Node>,
    pub regions: Vec<Region>,
    pub logs: VecDeque<LogLine>,
}

pub const SERVICES: &[(&str, &str)] = &[
    ("api-gateway", "eu-west"),
    ("auth", "eu-west"),
    ("orders", "us-east"),
    ("payments", "us-east"),
    ("search", "ap-south"),
    ("inventory", "us-west"),
    ("notifier", "ap-south"),
    ("ledger", "us-west"),
];

impl Sim {
    pub fn new() -> Self {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let services = SERVICES
            .iter()
            .map(|&(name, region)| Service {
                name,
                region,
                load: 0.3,
                replicas: 2 + rng.below(5),
                phase: rng.unit() * 100.0,
            })
            .collect();
        let nodes = (1..=16)
            .map(|i| Node {
                name: format!("node-{i:02}"),
                heat: Series::with_capacity(HISTORY),
                phase: rng.unit() * 100.0,
            })
            .collect();
        let regions = [
            ("eu-west", 24.0),
            ("us-east", 31.0),
            ("us-west", 38.0),
            ("ap-south", 52.0),
        ]
        .into_iter()
        .map(|(name, base)| Region {
            name,
            base,
            latency: base,
        })
        .collect();

        let gauges = [
            ("Error budget", true),
            ("Cache hits", true),
            ("Queue depth", false),
            ("Disk usage", false),
        ]
        .map(|(name, higher_is_better)| Indicator {
            name,
            value: 0.0,
            higher_is_better,
        })
        .into();

        let mut sim = Sim {
            rng,
            tick: 0,
            paused: false,
            incident: 0.0,
            wander: 0.0,
            budget: 0.84,
            cpu: Series::with_capacity(HISTORY),
            memory: Series::with_capacity(HISTORY),
            requests: Series::with_capacity(HISTORY),
            errors: Series::with_capacity(HISTORY),
            p50: Series::with_capacity(HISTORY),
            p99: Series::with_capacity(HISTORY),
            gauges,
            services,
            nodes,
            regions,
            logs: VecDeque::new(),
        };

        // Warm up, so the charts are not empty when the application starts.
        for _ in 0..HISTORY {
            sim.step();
        }
        sim
    }

    pub fn metric(&self, metric: Metric) -> &Series {
        match metric {
            Metric::Cpu => &self.cpu,
            Metric::Memory => &self.memory,
            Metric::Requests => &self.requests,
            Metric::Errors => &self.errors,
        }
    }

    /// Simulated wall clock, as `HH:MM:SS`.
    pub fn clock(&self) -> String {
        let seconds = 9 * 3600 + self.tick * STEP_MS / 1000;
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600 % 24,
            seconds / 60 % 60,
            seconds % 60
        )
    }

    /// Something, somewhere, just went very wrong.
    pub fn trigger_incident(&mut self) {
        self.incident = 1.0;
        self.log(
            Level::Error,
            "payments",
            "upstream timeout: payments → ledger (5000ms)".into(),
        );
        self.log(
            Level::Error,
            "ledger",
            "connection pool exhausted (64/64 in use)".into(),
        );
    }

    pub fn log(&mut self, level: Level, source: &'static str, message: String) {
        if self.logs.len() == MAX_LOGS {
            self.logs.pop_front();
        }
        self.logs.push_back(LogLine {
            time: self.clock(),
            level,
            source,
            message,
        });
    }

    /// Advances the simulation by one step.
    ///
    /// Returns the number of log lines added.
    pub fn step(&mut self) -> usize {
        self.tick += 1;
        let t = self.tick as f32;
        let incident = self.incident;
        self.incident *= 0.992;
        if self.incident < 0.02 {
            self.incident = 0.0;
        }

        // A slow random walk on top of a couple of sine waves gives a
        // pleasantly organic curve.
        self.wander = (self.wander + self.rng.signed() * 0.04).clamp(-0.25, 0.25);
        let load =
            (0.5 + 0.25 * (t / 37.0).sin() + 0.12 * (t / 7.3).sin() + self.wander).clamp(0.05, 1.0);

        let cpu = 22.0 + 48.0 * load + 5.0 * self.rng.signed() + 30.0 * incident;
        self.cpu.push(cpu.clamp(1.0, 100.0).into());

        let memory = 58.0 + 9.0 * (t / 90.0).sin() + 0.8 * self.rng.signed() + 14.0 * incident;
        self.memory.push(memory.clamp(1.0, 100.0).into());

        let requests = (900.0 + 1600.0 * load + 90.0 * self.rng.signed()) * (1.0 - 0.6 * incident);
        self.requests.push(requests.max(0.0).into());

        let errors = 0.25 + 0.2 * self.rng.unit() + 7.5 * incident * (0.7 + 0.3 * self.rng.unit());
        self.errors.push(errors.into());
        self.budget = (self.budget - errors * 0.0004 + 0.0002).clamp(0.0, 1.0);

        let p50 = 14.0 + 12.0 * load + 2.0 * self.rng.signed() + 45.0 * incident;
        // Smooth the tail latency a bit, so the curve is not just noise.
        let target = p50 * 2.2 + 8.0 + 190.0 * incident;
        let last = self.p99.last().unwrap_or(0.0) as f32;
        let p99 = last + (target - last) * 0.3 + 2.5 * self.rng.signed();
        self.p50.push(p50.into());
        self.p99.push(p99.into());

        for (i, service) in self.services.iter_mut().enumerate() {
            let wave = 0.5 + 0.35 * ((t + service.phase) / (11.0 + 3.0 * i as f32)).sin();
            let hit = if matches!(service.name, "payments" | "ledger") {
                incident * 0.8
            } else {
                incident * 0.25
            };
            let target = (wave * (0.5 + load * 0.6) + hit).clamp(0.02, 1.0);
            service.load += (target - service.load) * 0.2;
        }

        for (i, node) in self.nodes.iter_mut().enumerate() {
            let wave = ((t + node.phase) / (9.0 + i as f32)).sin() * 0.5 + 0.5;
            let burst = ((t / 23.0) - i as f32 * 0.45).sin().max(0.0).powi(3);
            let heat =
                0.15 + 0.3 * wave * load + 0.5 * burst + 0.5 * incident + 0.05 * self.rng.unit();
            node.heat.push(heat.clamp(0.0, 1.0).into());
        }

        for region in &mut self.regions {
            let target = region.base * (0.8 + 0.5 * load) + 160.0 * incident;
            region.latency += (target - region.latency) * 0.15 + self.rng.signed();
        }

        self.update_gauges();

        self.chatter(incident)
    }

    /// Stores the current value of each indicator, in the order they were created.
    fn update_gauges(&mut self) {
        let t = self.tick as f32;
        let cache = 0.93 + 0.05 * (t / 50.0).sin() - 0.35 * self.incident;
        let cpu = self.cpu.last().unwrap_or(0.0) as f32;
        let queue = cpu / 100.0 * 0.5 + 0.45 * self.incident;
        let disk = (0.43 + t * 2e-5).min(0.95);
        let values = [
            self.budget,
            cache.clamp(0.0, 1.0),
            queue.clamp(0.0, 1.0),
            disk,
        ];
        for (gauge, value) in self.gauges.iter_mut().zip(values) {
            gauge.value = value;
        }
    }

    /// Maybe emits some log lines.
    fn chatter(&mut self, incident: f32) -> usize {
        let mut added = 0;

        if self.rng.unit() < 0.3 + incident * 0.4 {
            let roll = self.rng.unit();
            let service = self.rng.pick(SERVICES).0;
            let rng = &mut self.rng;
            let (level, message) = if roll < 0.04 + incident * 0.5 {
                let message = match rng.below(4) {
                    0 => format!("upstream timeout after {}ms", 3000 + rng.below(4000)),
                    1 => "circuit breaker OPEN for ledger".to_string(),
                    2 => format!("request failed: 503 from pool-{}", rng.below(8)),
                    _ => format!("deadline exceeded ({} retries)", 1 + rng.below(3)),
                };
                (Level::Error, message)
            } else if roll < 0.16 + incident * 0.7 {
                let message = match rng.below(4) {
                    0 => format!("slow query ({}ms) on orders_by_user", 400 + rng.below(900)),
                    1 => format!("retrying upstream (attempt {}/3)", 1 + rng.below(3)),
                    2 => format!("queue depth at {}%", 70 + rng.below(30)),
                    _ => format!("GC pause of {}ms", 80 + rng.below(200)),
                };
                (Level::Warn, message)
            } else {
                let message = match rng.below(6) {
                    0 => format!(
                        "GET /api/v2/orders/{} 200 in {}ms",
                        1000 + rng.below(9000),
                        4 + rng.below(40)
                    ),
                    1 => format!("POST /api/v2/checkout 201 in {}ms", 20 + rng.below(90)),
                    2 => format!("cache hit ratio {:.1}%", 91.0 + rng.unit() * 8.0),
                    3 => format!(
                        "autoscaler: pool-{} scaled to {} replicas",
                        rng.below(8),
                        2 + rng.below(9)
                    ),
                    4 => format!("healthcheck ok ({} nodes)", 16),
                    _ => format!("flushed {} events to cold storage", 500 + rng.below(4000)),
                };
                (Level::Info, message)
            };
            self.log(level, service, message);
            added += 1;
        }

        added
    }
}
