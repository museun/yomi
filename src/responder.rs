use std::sync::{Arc, Mutex};

use crate::irc::{Message, Response};

// TODO is this abstraction needed?
pub trait Responder: Send + Sync {
    fn send(&self, response: Response);
    fn reply(&self, msg: &Message, data: String) {
        self.send(Response::Reply {
            channel: msg.channel.clone(),
            msg_id: msg.msg_id.clone(),
            data,
        });
    }
    fn error(&self, msg: &Message, data: String) {
        self.send(Response::Error {
            channel: msg.channel.clone(),
            data,
        });
    }
}

#[derive(Clone, Default)]
pub struct ResponderCollector {
    inner: Arc<Mutex<Vec<Response>>>,
}

impl ResponderCollector {
    pub fn drain(&self) -> Vec<Response> {
        std::mem::take(&mut *self.inner.lock().unwrap())
    }
}

impl Responder for ResponderCollector {
    fn send(&self, response: Response) {
        self.inner.lock().unwrap().push(response);
    }
}

#[derive(Clone, Debug)]
pub struct ResponderChannel {
    tx: flume::Sender<Response>,
}

impl ResponderChannel {
    pub const fn new(tx: flume::Sender<Response>) -> Self {
        Self { tx }
    }
}

impl Responder for ResponderChannel {
    fn send(&self, response: Response) {
        _ = self.tx.send(response)
    }
}
