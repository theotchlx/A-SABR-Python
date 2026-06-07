use pyo3::{exceptions::PyBaseException, prelude::*};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use a_sabr::{
    contact_manager::legacy::evl::EVLManager,
    contact_plan::{asabr_file_lexer::FileLexer, from_asabr_lexer::ASABRContactPlan},
    node_manager::none::NoManagement,
    routing::{aliases::*, Router},
    types::{Date, NodeID},
    vertex::Vertex,
};

use crate::{py_asabr_bundle::PyAsabrBundle, py_asabr_contact::PyAsabrContact};

// NOT thread-safe
#[pyclass(name = "AsabrRouter", unsendable)]
pub struct PyAsabrRouter {
    nodes_id_map: HashMap<String, NodeID>,
    all_contacts: Vec<PyAsabrContact>,
    router: Box<dyn Router<NoManagement, EVLManager>>,
}

fn make_nodes_id_map(vertices: &Vec<Vertex<NoManagement>>) -> HashMap<String, NodeID> {
    let mut nodes_id_map = HashMap::new();

    for vertex in vertices {
        match vertex {
            Vertex::INode(node) | Vertex::ENode(node) => {
                nodes_id_map.insert(node.get_node_name(), node.get_node_id());
            }
            Vertex::VNode(_) => {}
        }
    }

    nodes_id_map
}

#[pymethods]
impl PyAsabrRouter {
    #[new]
    fn new(asabr_contact_plan_filepath: &str, router_type: &str) -> PyResult<Self> {
        let file = File::open(asabr_contact_plan_filepath).unwrap();
        let lines: Vec<String> = BufReader::new(file).lines().map(|l| l.unwrap()).collect();
        let mut mylexer = FileLexer::new(lines.iter().map(|s| s.as_str()));
        let contact_plan =
            ASABRContactPlan::parse::<NoManagement, EVLManager>(&mut mylexer, None, None);

        match contact_plan {
            Ok(cp) => {
                let nodes_id_map = make_nodes_id_map(&cp.vertices);
                let all_contacts: Vec<PyAsabrContact> = cp
                    .contacts
                    .iter()
                    .map(|c| {
                        PyAsabrContact::from_raw(
                            c.info.tx_node_id,
                            c.info.rx_node_id,
                            c.info.start,
                            c.info.end,
                        )
                    })
                    .collect();
                let router = build_generic_router::<NoManagement, EVLManager>(
                    router_type,
                    cp,
                    Some(SpsnOptions {
                        check_priority: false,
                        check_size: true,
                        max_entries: 10,
                    }),
                )
                .map_err(|e| {
                    PyErr::new::<PyBaseException, _>(format!("[A-SABR][Router] Build error: {}", e))
                })?;

                Ok(Self {
                    nodes_id_map,
                    all_contacts,
                    router,
                })
            }
            Err(err) => Err(PyErr::new::<PyBaseException, _>(format!(
                "[A-SABR][ContactPlan] Parse error: {}",
                err
            ))),
        }
    }

    fn route(
        &mut self,
        source: NodeID,
        bundle: PyAsabrBundle,
        curr_time: Date,
        excluded_nodes: Vec<NodeID>,
    ) -> Vec<(PyAsabrContact, Vec<NodeID>)> {
        let bundle = bundle.to_native_bundle();

        if let Ok(Some(routing_output)) =
            self.router
                .route(source, &bundle, curr_time, &excluded_nodes)
        {
            let mut py_routing_output = Vec::new();

            for (contact, reachable_nodes) in routing_output.first_hops.values() {
                py_routing_output.push((
                    PyAsabrContact::from_native_contact(contact),
                    reachable_nodes
                        .iter()
                        .map(|stage_rc| stage_rc.borrow().to_node)
                        .collect(),
                ));
            }

            py_routing_output
        } else {
            Vec::new()
        }
    }

    fn get_node_id(&self, node_name: &str) -> PyResult<NodeID> {
        let result = self.nodes_id_map.get(node_name);

        if let Some(node_id) = result {
            Ok(*node_id)
        } else {
            Err(PyErr::new::<PyBaseException, _>(format!(
                "Node '{}' unknown",
                node_name
            )))
        }
    }

    /// Return all known nodes as a dict mapping node name to A-SABR node ID.
    fn get_all_nodes(&self) -> HashMap<String, NodeID> {
        self.nodes_id_map.clone()
    }

    /// Return all contacts from the contact plan.
    fn get_all_contacts(&self) -> Vec<PyAsabrContact> {
        self.all_contacts.clone()
    }
}
