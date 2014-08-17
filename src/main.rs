extern crate track_visulisation;
extern crate osmxml;

use std::collections::HashMap;
use osmxml::{Osm, OsmElement, Relation, Way, Node};
use track_visulisation::{Vertex, Face};

fn expand_element(elem: &OsmElement, elements: &HashMap<int, OsmElement>) -> Vec<(f64, f64)> {
    let refs = match *elem {
        Relation {members: ref m, ..} => m,
        Way {nodes: ref n, ..} => n,
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
    println!("{}", Vertex(10.0, 10.0, 10.0).to_string());
    println!("{}", Face(vec!(-1, -2, -3, -4)).to_string());
}
