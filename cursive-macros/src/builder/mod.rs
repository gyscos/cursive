#[cfg(feature = "builder")]
include!("real_mod.rs");

#[cfg(not(feature = "builder"))]
include!("dummy_mod.rs");
