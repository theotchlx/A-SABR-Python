use pyo3::prelude::*;
use std::{cell::RefCell, rc::Rc};

use a_sabr::{
    contact::Contact,
    contact_manager::legacy::evl::EVLManager,
    node_manager::none::NoManagement,
    types::{Date, NodeID},
};

#[pyclass(name = "AsabrContact")]
#[derive(Clone)]
pub struct PyAsabrContact {
    #[pyo3(get)]
    contact_id: usize,
    #[pyo3(get)]
    tx_node: NodeID,
    #[pyo3(get)]
    rx_node: NodeID,
    #[pyo3(get)]
    start_time: Date,
    #[pyo3(get)]
    end_time: Date,
}

impl PyAsabrContact {
    pub fn from_raw(tx_node: NodeID, rx_node: NodeID, start_time: Date, end_time: Date) -> Self {
        Self {
            contact_id: 0,
            tx_node,
            rx_node,
            start_time,
            end_time,
        }
    }

    pub fn from_native_contact(contact: &Rc<RefCell<Contact<NoManagement, EVLManager>>>) -> Self {
        let contact_id = Rc::as_ptr(contact) as usize;
        let contact = contact.borrow();

        Self {
            contact_id,
            tx_node: contact.get_tx_node_id(),
            rx_node: contact.get_rx_node_id(),
            start_time: contact.info.start,
            end_time: contact.info.end,
        }
    }
}
