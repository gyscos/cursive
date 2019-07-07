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
/// Remark: it is weird that you cannot name the variables in the enum functions to
/// indicate the role they have.
///
pub enum ModelEvent {
    // the metadata events
    GetRecords(mpsc::Sender<ResponseMeta>),
    GetFields(String, mpsc::Sender<ResponseMeta>),
    // the data events
    GetFirst(String, mpsc::Sender<Response>),
    GetNext(String, Data, mpsc::Sender<Response>),
    GetPrev(String, Data, mpsc::Sender<Response>),
    GetLast(String, mpsc::Sender<Response>),
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
    pub first_item: bool,
    pub last_item: bool,
    pub data: Vec<String>,
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

    pub fn get_prev(
        &mut self,
        record_name: String,
        current: Data,
    ) -> Response {
        if record_name != "CustList" {
            panic!("Record name not valid: {}", record_name);
        }
        let (sender, receiver) = mpsc::channel::<Response>();
        self.request
            .send(ModelEvent::GetPrev("CustList".to_string(), current, sender))
            .unwrap();
        receiver.recv().unwrap()
    }

    pub fn get_last(&mut self, record_name: String) -> Response {
        if record_name != "CustList" {
            panic!("Record name not valid: {}", record_name);
        }
        let (sender, receiver) = mpsc::channel::<Response>();
        self.request
            .send(ModelEvent::GetLast("CustList".to_string(), sender))
            .unwrap();
        receiver.recv().unwrap()
    }
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
        let counter = 0;
        let customer = self.customers.get(counter);
        let customer = match customer {
            Some(cust) => cust,
            None => return None,
        };
        let mut data = Vec::new();
        data.push(customer.name.clone());
        data.push(customer.tel_nr.clone());
        Some(Data {
            first_item: counter == 0,
            last_item: counter == self.customers.len() - 1,
            data,
        })
    }

    fn get_next(&self, current: Data) -> Option<Data> {
        let name = current.data.get(0);
        let name = match name {
            Some(nm) => nm.to_string(),
            None => return None,
        };
        let mut next = false;
        let mut data = Vec::new();
        for (counter, customer) in self.customers.iter().enumerate() {
            if next {
                data.push(customer.name.clone());
                data.push(customer.tel_nr.clone());
                return Some(Data {
                    first_item: counter == 0,
                    last_item: counter == self.customers.len() - 1,
                    data,
                });
            }
            if customer.name == name {
                next = true;
            }
        }
        // not found
        None
    }

    fn get_prev(&self, current: Data) -> Option<Data> {
        let name = current.data.get(0);
        let name = match name {
            Some(nm) => nm.to_string(),
            None => return None,
        };
        let mut data = Vec::new();
        for (counter, customer) in self.customers.iter().enumerate() {
            if customer.name == name {
                if counter > 0 {
                    data.push(self.customers[counter - 1].name.clone());
                    data.push(self.customers[counter - 1].tel_nr.clone());
                    return Some(Data {
                        first_item: counter == 0,
                        last_item: counter == self.customers.len() - 1,
                        data,
                    });
                }
            }
        }
        // If not found
        None
    }

    fn get_last(&self) -> Option<Data> {
        if self.customers.len() == 0 {
            return None;
        }
        let counter = self.customers.len() - 1;
        let customer = self.customers.get(counter);
        let customer = match customer {
            Some(cust) => cust,
            None => return None,
        };
        let mut data = Vec::new();
        data.push(customer.name.clone());
        data.push(customer.tel_nr.clone());
        Some(Data {
            first_item: counter == 0,
            last_item: counter == self.customers.len() - 1,
            data,
        })
    }

    fn get_all(&self) -> Vec<Data> {
        let mut all = vec![];
        for (counter, customer) in self.customers.iter().enumerate() {
            let mut data = vec![];
            data.push(customer.name.clone());
            data.push(customer.tel_nr.clone());
            all.push(Data {
                first_item: counter == 0,
                last_item: counter == self.customers.len() - 1,
                data,
            });
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
                    log::info!(
                        "[UiModel GetRecords] Not implementted: sending None"
                    );
                    sender.send(ResponseMeta::DataFieldNames(None)).unwrap();
                }
                ModelEvent::GetFields(record_name, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run GetFields] {}", msg);
                        sender.send(ResponseMeta::ErrorResponse(msg)).unwrap();
                    } else {
                        let msg = "Sending datafields".to_string();
                        log::info!("[UiModel::run GetFields] {}", msg);
                        sender
                            .send(ResponseMeta::DataFields(self.data_fields()))
                            .unwrap();
                    }
                }
                ModelEvent::GetFirst(record_name, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run GetFirst] {}", msg);
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    } else {
                        let msg = "Sending first data".to_string();
                        log::info!("[UiModel::run GetFirst] {}", msg);
                        sender
                            .send(Response::DataResponse(self.get_first()))
                            .unwrap();
                    }
                }
                ModelEvent::GetNext(record_name, current, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run GetNext] {}", msg);
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    } else {
                        let msg = "Sending next data".to_string();
                        log::info!("[UiModel::run GetNext] {}", msg);
                        sender
                            .send(Response::DataResponse(
                                self.get_next(current),
                            ))
                            .unwrap();
                    }
                }
                ModelEvent::GetPrev(record_name, current, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run GetPrev] {}", msg);
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    } else {
                        let msg = "Sending next data".to_string();
                        log::info!("[UiModel::run GetPrev] {}", msg);
                        sender
                            .send(Response::DataResponse(
                                self.get_prev(current),
                            ))
                            .unwrap();
                    }
                }
                ModelEvent::GetLast(record_name, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run GetLast] {}", msg);
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    } else {
                        let msg = "Sending first data".to_string();
                        log::info!("[UiModel::run GetLast] {}", msg);
                        sender
                            .send(Response::DataResponse(self.get_last()))
                            .unwrap();
                    }
                }
                ModelEvent::GetAll(record_name, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run GetAll] {}", msg);
                        sender.send(Response::ErrorResponse(msg)).unwrap();
                    } else {
                        let msg = "Sending all data".to_string();
                        log::info!("[UiModel::run GetAll] {}", msg);
                        sender
                            .send(Response::SelectDataResponse(self.get_all()))
                            .unwrap();
                    }
                }
                ModelEvent::Update(record_name, before, after, sender) => {
                    if record_name != "CustList" {
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run Update] {}", msg);
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
                        let msg =
                            "Received non-custlist record_name".to_string();
                        log::info!("[UiModel::run Remove] {}", msg);
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

use cursive::event::Key;
use cursive::traits::*;
use cursive::views;
use cursive::views::Dialog;
use cursive::views::EditView;
use cursive::views::TextView;
use cursive::Cursive;

const ID_DETAIL: &str = "id_detail";
const BTN_FIRST: &str = "First";
const BTN_LAST: &str = "Last";
const BTN_NEXT: &str = "Next";
const BTN_PREV: &str = "Prev";
const BTN_QUIT: &str = "Quit";
///
pub struct Ui {
    siv: cursive::Cursive,
    model: UiModel<ModelEvent>,
    fields: DataFields,
    data: Option<Data>,
    receiver: mpsc::Receiver<UiMessage>,
}

/// message from callback functions to Ui struct
enum UiMessage {
    First,
    Next,
    Prev,
    Last,
    Quit,
}

impl Ui {
    pub fn new(model: UiModel<ModelEvent>) -> Ui {
        let fields = DataFields {
            identifier: None,
            fields: vec![],
        };
        let data = None;
        let (sender, receiver) = mpsc::channel::<UiMessage>();
        let mut ui = Ui {
            siv: cursive::Cursive::default(),
            model,
            fields,
            data,
            receiver,
        };
        ui.siv.set_user_data(sender);
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

    fn execute_get_first() -> Box<dyn Fn(&mut Cursive) -> ()> {
        Box::new(|s: &mut Cursive| {
            let sender: &mut mpsc::Sender<UiMessage> = match s.user_data() {
                Some(sender) => sender,
                None => return,
            };
            match sender.send(UiMessage::First) {
                Ok(_) => return,
                Err(e) => {
                    log::error!(
                        "[button first] Sending for first data failed: {}.",
                        e
                    );
                }
            };
        })
    }

    fn execute_get_next() -> Box<dyn Fn(&mut Cursive) -> ()> {
        Box::new(|s: &mut Cursive| {
            let sender: &mut mpsc::Sender<UiMessage> = match s.user_data() {
                Some(sender) => sender,
                None => return,
            };
            match sender.send(UiMessage::Next) {
                Ok(_) => return,
                Err(e) => {
                    log::error!(
                        "[button next] Sending for next data failed: {}.",
                        e
                    );
                }
            };
        })
    }

    fn execute_get_prev() -> Box<dyn Fn(&mut Cursive) -> ()> {
        Box::new(|s: &mut Cursive| {
            let sender: &mut mpsc::Sender<UiMessage> = match s.user_data() {
                Some(sender) => sender,
                None => return,
            };
            match sender.send(UiMessage::Prev) {
                Ok(_) => return,
                Err(e) => {
                    log::error!(
                        "[button next] Sending for prev data failed: {}.",
                        e
                    );
                }
            };
        })
    }

    fn execute_get_last() -> Box<dyn Fn(&mut Cursive) -> ()> {
        Box::new(|s: &mut Cursive| {
            let sender: &mut mpsc::Sender<UiMessage> = match s.user_data() {
                Some(sender) => sender,
                None => return,
            };
            match sender.send(UiMessage::Last) {
                Ok(_) => return,
                Err(e) => {
                    log::error!(
                        "[button next] Sending for last data failed: {}.",
                        e
                    );
                }
            };
        })
    }

    fn execute_quit() -> Box<dyn Fn(&mut Cursive) -> ()> {
        Box::new(|s: &mut Cursive| {
            let sender: &mut mpsc::Sender<UiMessage> = match s.user_data() {
                Some(sender) => sender,
                None => return,
            };
            match sender.send(UiMessage::Quit) {
                Ok(_) => return,
                Err(e) => {
                    log::error!(
                        "[button next] Sending for quit failed: {}.",
                        e
                    );
                }
            };
        })
    }

    fn define_data_dialog(&mut self) -> impl cursive::view::View {
        Dialog::new()
            .title("Detail View")
            .padding((1, 1, 1, 0))
            .content(self.define_data_view())
            .button(BTN_QUIT, Ui::execute_quit())
            .button(BTN_FIRST, Ui::execute_get_first())
            .button(BTN_NEXT, Ui::execute_get_next())
            .button(BTN_PREV, Ui::execute_get_prev())
            .button(BTN_LAST, Ui::execute_get_last())
            .with_id(ID_DETAIL)
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
        let response = self.model.get_first("CustList".to_string());
        match response {
            Response::DataResponse(Some(data)) => {
                log::info!("Received data response to get first");
                Some(data)
            }
            Response::DataResponse(None) => {
                log::info!("Received none response to get first");
                None
            }
            Response::ErrorResponse(err) => {
                log::error!("{}", err);
                None
            }
            _ => {
                log::warn!("unexpected response to get first");
                None
            }
        }
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
            Response::DataResponse(Some(data)) => {
                log::info!("Received data response to get next");
                Some(data)
            }
            Response::DataResponse(None) => {
                log::info!("Received none response to get next");
                None
            }
            Response::ErrorResponse(err) => {
                log::error!("{}", err);
                None
            }
            _ => {
                log::warn!("unexpected response to get next");
                None
            }
        }
    }

    fn get_last(&mut self) -> Option<Data> {
        let response = self.model.get_last("CustList".to_string());
        match response {
            Response::DataResponse(Some(data)) => {
                log::info!("Received data response to get last");
                Some(data)
            }
            Response::DataResponse(None) => {
                log::info!("Received none response to get last");
                None
            }
            Response::ErrorResponse(err) => {
                log::error!("{}", err);
                None
            }
            _ => {
                log::warn!("unexpected response to get last");
                None
            }
        }
    }

    fn get_prev(&mut self) -> Option<Data> {
        let before: Data;
        if let Some(data) = &self.data {
            before = data.clone();
        } else {
            return None;
        }

        let response = self.model.get_prev("CustList".to_string(), before);
        match response {
            Response::DataResponse(Some(data)) => {
                log::info!("Received data response to get last");
                Some(data)
            }
            Response::DataResponse(None) => {
                log::info!("Received none response to get last");
                None
            }
            Response::ErrorResponse(err) => {
                log::error!("{}", err);
                None
            }
            _ => {
                log::warn!("unexpected response to get last");
                None
            }
        }
    }

    fn fields_refresh(&mut self) {
        let d = &self.data;
        if let Some(data) = d {
            if data.data.len() < self.fields.fields.len() {
                log::error!("[Fields refresh] Error in Data.data length. Len fields = {}, len of data = {}",
                        data.data.len(), self.fields.fields.len());
                return;
            }
        }
        for (counter, field) in self.fields.fields.iter().enumerate() {
            let id = format!("id_{}", field.name);
            self.siv.call_on_id(id.as_str(), |view: &mut EditView| {
                match d {
                    Some(d) => view.set_content(d.data[counter].clone()),
                    None => view.set_content(String::new()),
                };
            });
        }
        let mut dialog = self.siv.find_id::<Dialog>(ID_DETAIL).unwrap();
        for button in dialog.buttons_mut() {
            match d {
                None => button.disable(),
                Some(d) => match button.label() {
                    BTN_FIRST => {
                        if d.first_item == true {
                            button.disable();
                        } else {
                            button.enable();
                        }
                    }
                    BTN_LAST => {
                        if d.last_item == true {
                            button.disable();
                        } else {
                            button.enable();
                        }
                    }
                    BTN_NEXT => {
                        if d.last_item {
                            button.disable()
                        } else {
                            button.enable()
                        }
                    }
                    BTN_PREV => {
                        if d.first_item {
                            button.disable()
                        } else {
                            button.enable()
                        }
                    }
                    _ => (),
                },
            }
        }
    }

    fn init_ui(&mut self) {
        log::info!("Starting init for user interface");
        // Load first data and field definition
        self.fields = self.get_fields();
        self.data = self.get_first();
        // define the view showing the data
        self.siv.add_global_callback('q', Cursive::quit);
        // TODO: These keys don't seem to work: from execute, but do work if copied in directly
        self.siv
            .add_global_callback(Key::PageDown, |s: &mut Cursive| {
                let sender: &mut mpsc::Sender<UiMessage> = match s.user_data()
                {
                    Some(sender) => sender,
                    None => return,
                };
                match sender.send(UiMessage::Next) {
                    Ok(_) => return,
                    Err(e) => {
                        log::error!(
                            "[button next] Sending for next data failed: {}.",
                            e
                        );
                    }
                };
            });
        self.siv
            .add_global_callback(Key::PageUp, |s: &mut Cursive| {
                let sender: &mut mpsc::Sender<UiMessage> = match s.user_data()
                {
                    Some(sender) => sender,
                    None => return,
                };
                match sender.send(UiMessage::Prev) {
                    Ok(_) => return,
                    Err(e) => {
                        log::error!(
                            "[button next] Sending for prev data failed: {}.",
                            e
                        );
                    }
                };
            });
        self.siv
            .add_global_callback('`', Cursive::toggle_debug_console);
        let dialog = self.define_data_dialog();
        self.siv.add_layer(dialog);
    }

    pub fn run(&mut self) {
        self.init_ui();
        //self.siv.run();
        // We need to receive messages in between steps
        while self.step() {
            while let Some(msg) = self.receiver.try_iter().next() {
                match msg {
                    UiMessage::First => {
                        let data = self.get_first();
                        match data {
                            None => (),
                            Some(d) => self.data = Some(d),
                        }
                        self.fields_refresh();
                    }
                    UiMessage::Next => {
                        let data = self.get_next();
                        match data {
                            None => (),
                            Some(d) => self.data = Some(d),
                        }
                        self.fields_refresh();
                    }
                    UiMessage::Last => {
                        let data = self.get_last();
                        match data {
                            None => (),
                            Some(d) => self.data = Some(d),
                        }
                        self.fields_refresh();
                    }
                    UiMessage::Prev => {
                        let data = self.get_prev();
                        match data {
                            None => (),
                            Some(d) => self.data = Some(d),
                        }
                        self.fields_refresh();
                    }
                    UiMessage::Quit => {
                        self.siv.quit();
                        self.fields_refresh();
                    }
                }
            }
            self.siv.refresh();
        }
    }

    pub fn step(&mut self) -> bool {
        if !self.siv.is_running() {
            return false;
        }

        self.siv.step();
        true
    }
}

pub fn main() {
    cursive::logger::init();
    log::set_max_level(log::LevelFilter::Info);
    let mut cust_list = CustList::new();
    let ui_model = cust_list.ui_model();
    thread::spawn(move || {
        cust_list.run();
    });
    let mut ui = Ui::new(ui_model);
    ui.run();
}
