use lazy_static::lazy_static;

use std::sync::Mutex;

use tokio::sync::mpsc::{channel, Receiver, Sender};

pub enum Messages {
    String(String),
    OSMessage(String),
    DriverMessage(String),
    GlobalMessages(String),
    FileWrite(String, String),
    FileRead(String),
}

pub struct CommsLink {
    pub tx: Sender<Messages>,
    pub rx: Receiver<Messages>,
}

impl CommsLink {
    pub fn new_pair() -> (Self, Self) {
        let (tx1, rx1) = channel(100);
        let (tx2, rx2) = channel(100);

        (Self::from_channels(tx1, rx2), Self::from_channels(tx2, rx1))
    }

    pub fn from_channels(tx: Sender<Messages>, rx: Receiver<Messages>) -> Self {
        Self { tx, rx }
    }
}

pub struct GlobalLink {
    sender: Mutex<Option<Sender<Messages>>>,
}

impl GlobalLink {
    pub fn new_link() -> Self {
        Self {
            sender: Mutex::new(None),
        }
    }

    pub fn set_tx_value(&self, new_sender: Sender<Messages>) {
        let mut locked = self.sender.lock().unwrap();
        *locked = Some(new_sender);
    }

    pub fn write_message(&self, message: String) {
        let mut result = self.sender.lock().unwrap();

        let channel = match *result {
            Some(ref mut val) => val,
            _ => return,
        };

        channel.blocking_send(Messages::GlobalMessages(message)).unwrap()
    }
}

lazy_static! {
    pub static ref GLOBAL_COMMS: GlobalLink = GlobalLink::new_link();
}
