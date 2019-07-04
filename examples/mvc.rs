//////////////////////////////////////////////////
//// Start of general model definitions
//////////////////////////////////////////////////
use std::sync::mpsc;

/// The events a model can process
///
/// The event contains the channel through which to respond. The
/// requestor is responsible to creating the channel to receive a response.
///
/// The items in the enum represent the general actions any model should be
/// able to take when sending from a single source of data. For example
/// it could be a list of customers with name and address, or a list of
/// data as the result of a query.
///
/// The second `String`, for `GetNext` and `GetPrevious` is meant to be the identifier
/// of the latest data. The first String for all calls is meant to be the record_name.
/// Update has a before and after image of the data in that order.
/// Remove has the image of the data before removal.
///
/// Remark: it is weird that you cannot name the variables in the enum functions.
///
pub enum ModelEvent {
    // the metadata events
    GetRecords(mpsc::Sender<ResponseMeta>),
    GetFields(String, mpsc::Sender<ResponseMeta>),
    // the data events
    GetFirst(String, mpsc::Sender<Response>),
    GetNext(String, Data, mpsc::Sender<Response>),
    GetAll(String, mpsc::Sender<Response>),
    Update(String, Data, Data, mpsc::Sender<Response>),
    Remove(String, Data, mpsc::Sender<Response>),
    // miscelaneous
    Error(mpsc::RecvError), // not for sending to model
    Quit,
}

/// The enum that defines a response from the model to the requestor.
///
/// The data contains the actual data.
pub enum Response {
    DataResponse(Option<Data>),
    SelectDataResponse(Vec<Data>),
    UpdateOk(String),
    UpdateNok(String),
    RemoveOk(String),
    RemoveNok(String),
    ErrorResponse(String),
}

pub enum ResponseMeta {
    DataFields(DataFields),
    DataFieldNames(Option<Vec<String>>),
    ErrorResponse(String),
}

/// The UiModel contains the request field to send a request and the definition of the
/// datafields through a function.
///
/// The name UiModel represents the fact that this is *not* the model itself, but a
/// an accesspoint to the model for a requestor.
///
pub struct UiModel<T> {
    request: mpsc::Sender<T>,
}

impl UiModel<ModelEvent> {
    pub fn new(request: mpsc::Sender<ModelEvent>) -> UiModel<ModelEvent> {
        UiModel { request }
    }

    pub fn get_records(&mut self) -> ResponseMeta {
        let (sender, receiver) = mpsc::channel::<ResponseMeta>();
        self.request.send(ModelEvent::GetRecords(sender)).unwrap();
        receiver.recv().unwrap()
    }

    pub fn get_fields(&mut self, record_name: String) -> ResponseMeta {
        if record_name != "CustList" {
            panic!("Record name not valid: {}", record_name);
        }
        let (sender, receiver) = mpsc::channel::<ResponseMeta>();
        self.request
            .send(ModelEvent::GetFields("CustList".to_string(), sender))
            .unwrap();
        match receiver.recv() {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{}", e);
                panic!("Program ending due to channel receive error.");
            }
        }
    }

    pub fn get_first(&mut self, record_name: String) -> Response {
        if record_name != "CustList" {
            panic!("Record name not valid: {}", record_name);
        }
        let (sender, receiver) = mpsc::channel::<Response>();
        self.request
            .send(ModelEvent::GetFirst("CustList".to_string(), sender))
            .unwrap();
        receiver.recv().unwrap()
    }

    pub fn get_next(
        &mut self,
        record_name: String,
        current: Data,
    ) -> Response {
        if record_name != "CustList" {
            panic!("Record name not valid: {}", record_name);
        }
        let (sender, receiver) = mpsc::channel::<Response>();
        self.request
            .send(ModelEvent::GetNext("CustList".to_string(), current, sender))
            .unwrap();
        receiver.recv().unwrap()
    }
}

trait DataDescription {
    fn data_fields(&self) -> DataFields;
}

/// The description of each field is in this struct.
///
#[derive(Clone)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub secret: bool,
    pub multiline: bool,
}

/// The combination of fields that make up the data the model sends to the requestor.
///
/// The identifier is the name of the field that identifies a group of fields. The
/// reason this is not in Field, is because a field could be identifier in one group
/// of data, and not an identifier in the next group of data.
pub struct DataFields {
    pub identifier: Option<Field>,
    pub fields: Vec<Field>,
}

/// The concrete data as per a definition of DataFields.
#[derive(Clone)]
pub struct Data {
    pub data: Vec<String>,
}

//////////////////////////////////////////////////
//// End of general model definitions
//////////////////////////////////////////////////

//////////////////////////////////////////////////
//// Start of Specific model definitions
//////////////////////////////////////////////////

use std::thread;

struct Customer {
    name: String,
    tel_nr: String,
}

struct CustList {
    request_queue: mpsc::Receiver<ModelEvent>,
    request_sender: mpsc::Sender<ModelEvent>,
    customers: Vec<Customer>,
    identifier: Option<Field>,
    fields: Vec<Field>,
}

impl CustList {
    pub fn new() -> CustList {
        let fields: [Field; 2] = [
            Field {
                name: "custname".to_string(),
                label: "name".to_string(),
                secret: false,
                multiline: false,
            },
            Field {
                name: "telnr".to_string(),
                label: "telephone".to_string(),
                secret: false,
                multiline: false,
            },
        ];

        let (sender, receiver) = mpsc::channel::<ModelEvent>();
        let fields = fields.to_vec();
        let clist = CustList {
            customers: vec![],
            request_queue: receiver,
            request_sender: sender,
            identifier: Some(fields[0].clone()),
            fields,
        };
        CustList::preload(clist)
    }

    fn data_fields(&self) -> DataFields {
        DataFields {
            identifier: self.identifier.clone(),
            fields: self.fields.clone(),
        }
    }

    fn preload(mut clist: CustList) -> CustList {
        clist.customers.push(Customer {
            name: "John".to_string(),
            tel_nr: "3001".to_string(),
        });
        clist.customers.push(Customer {
            name: "Paul".to_string(),
            tel_nr: "3304".to_string(),
        });
        clist.customers.push(Customer {
            name: "George".to_string(),
            tel_nr: "3304".to_string(),
        });
        clist
    }

    fn get_first(&self) -> Option<Data> {
        let customer = self.customers.get(1);
        let customer = match customer {
            Some(cust) => cust,
            None => return None,
        };
        let mut data = Vec::new();
        data.push(customer.name.clone());
        data.push(customer.tel_nr.clone());
        Some(Data { data })
    }

    fn get_next(&self, current: Data) -> Option<Data> {
        let name = current.data.get(0);
        let name = match name {
            Some(nm) => nm,
            None => return None,
        };
        let mut next = false;
        let mut data = Vec::new();
        for customer in &self.customers {
            if next {
                data.push(customer.name.clone());
                data.push(customer.tel_nr.clone());
                return Some(Data { data });
            }
            if customer.name == name.clone() {
                next = true;
            }
        }
        // not found
        None
    }

    fn get_all(&self) -> Vec<Data> {
        let mut all = vec![];
        for customer in &self.customers {
            let mut data = vec![];
            data.push(customer.name.clone());
            data.push(customer.tel_nr.clone());
            all.push(Data { data });
        }
        all
    }

    fn update(
        &mut self,
        before: Customer,
        after: Customer,
    ) -> Result<String, String> {
        if before.name != after.name {
            return Err("name is identifier and cannot change.".to_string());
        }
        for cust_old in &mut self.customers {
            // remark: could check before against cust_old, but won't
            if cust_old.name == after.name {
                if cust_old.tel_nr != before.tel_nr {
                    let msg =
                        format!("Customer {} has stale data.", cust_old.name);
                    return Err(msg);
                }
                cust_old.tel_nr = after.tel_nr;
                let msg = format!("Customer {} updated.", after.name);
                return Ok(msg);
            }
        }
        let msg = format!("Customer {} not found.", after.name);
        Err(msg)
    }

    fn remove(&mut self, del_cust: Customer) -> Result<String, String> {
        let len = self.customers.len();
        self.customers.retain(|cust| cust.name != del_cust.name);

        if len == self.customers.len() {
            let msg = format!("Customer {} not found.", del_cust.name);
            return Err(msg);
        }
        let msg = format!("Customer {} removed.", del_cust.name);
        Ok(msg)
    }

    pub fn ui_model(&mut self) -> UiModel<ModelEvent> {
        UiModel {
            request: self.request_sender.clone(),
        }
    }

    fn run(&mut self) {
        loop {
            let result = self.request_queue.recv();
            let msg = match result {
                Ok(received) => received,
                Err(e) => ModelEvent::Error(e),
            };
            match msg {
                ModelEvent::GetRecords(sender) => {
                    sender.send(ResponseMeta::DataFieldNames(None)).unwrap();
                }
                ModelEvent::GetFields(record_name, sender) => {
                    if record_name != "CustList" {
                        sender
                            .send(ResponseMeta::ErrorResponse(
                                "Received non-custlist record_name"
                                    .to_string(),
                            ))
                            .unwrap();
                    } else {
                        sender
                            .send(ResponseMeta::DataFields(self.data_fields()))
                            .unwrap();
                    }
                }
                ModelEvent::GetFirst(record_name, sender) => {
                    if record_name != "CustList" {
                        sender
                            .send(Response::ErrorResponse(
                                "Received non-custlist record_name"
                                    .to_string(),
                            ))
                            .unwrap();
                    } else {
                        sender
                            .send(Response::DataResponse(self.get_first()))
                            .unwrap();
                    }
                }
                ModelEvent::GetNext(record_name, current, sender) => {
                    if record_name != "CustList" {
                        sender
                            .send(Response::ErrorResponse(
                                "Received non-custlist record_name"
                                    .to_string(),
                            ))
                            .unwrap();
                    } else {
                        sender
                            .send(Response::DataResponse(
                                self.get_next(current),
                            ))
                            .unwrap();
                    }
                }
                ModelEvent::GetAll(record_name, sender) => {
                    if record_name != "CustList" {
                        sender
                            .send(Response::ErrorResponse(
                                "Received non-custlist record_name"
                                    .to_string(),
                            ))
                            .unwrap();
                    } else {
                        sender
                            .send(Response::SelectDataResponse(self.get_all()))
                            .unwrap();
                    }
                }
                ModelEvent::Update(record_name, before, after, sender) => {
                    if record_name != "CustList" {
                        let msg = format!(
                            "[Update] record name {} unknown.",
                            record_name
                        );
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    }
                    if before.data.len() != 2 {
                        let msg = "before data is of invalid length";
                        sender
                            .send(Response::ErrorResponse(msg.to_string()))
                            .unwrap();
                    }
                    let name = before.data[0].clone();
                    let tel_nr = before.data[1].clone();
                    let before_customer = Customer { name, tel_nr };

                    let name = after.data[0].clone();
                    let tel_nr = after.data[1].clone();
                    let after_customer = Customer { name, tel_nr };

                    let result = self.update(before_customer, after_customer);
                    match result {
                        Ok(msg) => {
                            sender.send(Response::UpdateOk(msg)).unwrap();
                        }
                        Err(msg) => {
                            sender.send(Response::UpdateNok(msg)).unwrap();
                        }
                    }
                }
                ModelEvent::Remove(record_name, cust_del, sender) => {
                    if record_name != "CustList" {
                        let msg = format!(
                            "[Remove] record name {} unknown.",
                            record_name
                        );
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    }
                    if cust_del.data.len() != 2 {
                        let msg = "[Remove] customer data has wrong length.";
                        sender
                            .send(Response::ErrorResponse(msg.to_string()))
                            .unwrap();
                    }
                    let customer = Customer {
                        name: cust_del.data[0].clone(),
                        tel_nr: cust_del.data[1].clone(),
                    };
                    let result = self.remove(customer);
                    match result {
                        Ok(msg) => {
                            sender.send(Response::RemoveOk(msg)).unwrap();
                        }
                        Err(msg) => {
                            sender.send(Response::RemoveNok(msg)).unwrap();
                        }
                    }
                }
                ModelEvent::Quit => {
                    // Nothing to do at the moment
                }
                ModelEvent::Error(error) => {
                    panic!("Error receiving response: {}", error);
                }
            }
        }
    }
}

//////////////////////////////////////////////////
//// End of Specific model definitions
//////////////////////////////////////////////////

//////////////////////////////////////////////////
//// Start of Ui definitions
//////////////////////////////////////////////////

use cursive::traits::*;
use cursive::views;
use cursive::views::Dialog;
use cursive::views::EditView;
use cursive::views::TextView;
use cursive::Cursive;

///
pub struct Ui {
    siv: cursive::Cursive,
    model: UiModel<ModelEvent>,
    fields: DataFields,
    data: Option<Data>,
}

impl Ui {
    pub fn new(mut model: UiModel<ModelEvent>) -> Ui {
        let fields = DataFields {
            identifier: None,
            fields: vec![],
        };
        let data = None;
        let ui = Ui {
            siv: cursive::Cursive::default(),
            model,
            fields,
            data,
        };
        ui
    }

    fn get_fields(&mut self) -> DataFields {
        // send GetFields and process response
        let response = self.model.get_fields("CustList".to_string());
        match response {
            ResponseMeta::DataFields(data_fields) => data_fields,
            ResponseMeta::DataFieldNames(_) => {
                panic!(
                    "Receiving datafield names when datafields were expected"
                );
            }
            ResponseMeta::ErrorResponse(error) => {
                // Do something with the error response
                panic!("Do something with the error response: {}", error);
            }
        }
    }

    fn load_first(&mut self) -> Option<Data> {
        let response = self.model.get_first("CustList".to_string());
        match response {
            Response::DataResponse(Some(data)) => Some(data),
            Response::DataResponse(None) => None,
            Response::ErrorResponse(err) => {
                eprintln!("{}", err);
                None
            }
            _ => {
                // TODO: this should be in a status line
                eprintln!("unexpected response to load of data");
                None
            }
        }
    }

    fn define_data_dialog(&mut self) -> impl cursive::view::View {
        Dialog::new()
            .title("Detail View")
            .padding((1, 1, 1, 0))
            .content(self.define_data_view())
            .button("Quit (q)", |s| {
                s.quit();
            })
            // TODO: HOW to use Ui::get_next?
            .button("First", |_s| self.data = self.get_first())
            .button("Next", |_s| self.data = self.get_next())
    }

    fn define_data_view(&mut self) -> impl cursive::view::View {
        let mut view = views::LinearLayout::vertical();
        let mut maxlen: usize = 0;
        for field in &self.fields.fields {
            maxlen = if field.name.len() > maxlen {
                field.name.len()
            } else {
                maxlen
            }
        }
        if maxlen < 20 {
            maxlen = 20;
        }
        for (count, field) in self.fields.fields.iter().enumerate() {
            let text = format!("{}: ", field.name.clone());
            let id = format!("id_{}", field.name.clone());
            let fld_data: String;
            if let Some(d) = &self.data {
                fld_data = d.data[count].clone();
            } else {
                fld_data = String::new();
            }
            let mut hview = views::LinearLayout::horizontal();
            hview =
                hview.child(TextView::new(text).fixed_width(maxlen)).child(
                    EditView::new()
                        .content(fld_data)
                        .with_id(id)
                        .min_width(10),
                );
            view = view.child(hview);
        }
        /////// returning result
        view.fixed_width(30)
    }

    fn get_first(&mut self) -> Option<Data> {
        self.load_first()
    }

    fn get_next(&mut self) -> Option<Data> {
        let before: Data;
        if let Some(data) = &self.data {
            before = data.clone();
        } else {
            return None;
        }

        let response = self.model.get_next("CustList".to_string(), before);
        match response {
            Response::DataResponse(Some(data)) => Some(data),
            Response::DataResponse(None) => None,
            Response::ErrorResponse(err) => {
                eprintln!("{}", err);
                None
            }
            _ => {
                // TODO: this should be in a status line
                eprintln!("unexpected response to load of data");
                None
            }
        }
    }

    pub fn run(&mut self) {
        // define the view showing the data
        self.siv.add_global_callback('q', Cursive::quit);
        self.siv.add_layer(TextView::new("Hello World!"));
        let dialog = self.define_data_dialog();
        self.siv.add_layer(dialog);
        self.siv.run();
    }
}

pub fn main() {
    let mut cust_list = CustList::new();
    let ui_model = cust_list.ui_model();
    thread::spawn(move || {
        cust_list.run();
    });
    let mut ui = Ui::new(ui_model);
    ui.fields = ui.get_fields();
    ui.data = ui.get_first();
    ui.run();
}
