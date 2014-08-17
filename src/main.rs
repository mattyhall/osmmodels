extern crate osmxml;

use std::collections::HashMap;
use osmxml::{Osm, OsmElement, Relation, Way, Node};

fn expand_element(elem: &OsmElement, elements: &HashMap<int, OsmElement>) -> Vec<(f64, f64)> {
    let refs = match *elem {
        Relation {id: _, members: ref m, tags: _} => m,
        Way {id: _, nodes: ref n, tags: _} => n,
        Node {lat: lat, lng: lng, ..} => return vec!((lat, lng))
    };
    let mut latlngs = Vec::new(); 
    for r in refs.iter() {
        let vs = match elements.find(r) {
            Some(e) => expand_element(e, elements),
            None => fail!("Could not find element with id {}", r)
        };
        latlngs.push_all(vs.as_slice());
    }
    latlngs
}

fn main() {
    let path = &Path::new("spa.osm");
    let osm = Osm::new(path).unwrap();
    let track = "Ciruit de Spa Francorchamps".to_string();
    let relation = osm.elements.values().filter(|e| {
        match **e {
            Relation{id: _, members: _, tags: ref ts} => {
                ts.find(&"name".to_string()) == Some(&track)
            }
            _ => false
        }
    }).next().unwrap();
    println!("{}", expand_element(relation, &osm.elements));
}
