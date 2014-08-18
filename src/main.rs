extern crate track_visulisation;
extern crate osmxml;
extern crate cgmath;

use std::collections::HashMap;
use osmxml::{Osm, OsmElement, Relation, Way, Node};
use track_visulisation::{Wavefront};
use cgmath::{Vector2, EuclideanVector, Vector};

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

type Vec2 = Vector2<f64>;

fn top(w: &mut Wavefront, height: f64, a: Vec2, a1: Vec2, b: Vec2, b1: Vec2) {
    w.add_vertex(a.x, a.y, height);
    w.add_vertex(a1.x, a1.y, height);
    w.add_vertex(b1.x, b1.y, height);
    w.add_vertex(b.x, b.y, height);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn side(w: &mut Wavefront, height: f64, a: Vec2, b: Vec2) {
    w.add_vertex(a.x, a.y, height);
    w.add_vertex(b.x, b.y, height);
    w.add_vertex(b.x, b.y, 0.0);
    w.add_vertex(a.x, a.y, 0.0);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn to_wavefront(thickness: f64, height: f64, latlngs: Vec<(f64, f64)>) -> Wavefront {
    let mut iter = latlngs.iter().zip(latlngs.iter().skip(1));
    let mut w = Wavefront::new();
    for (&(ax, ay), &(bx, by)) in iter {
        let a = Vector2::new(ax, ay);
        let b = Vector2::new(bx, by);
        let ab = Vector2::new(bx - ax, by - ay).normalize();
        let p = Vector2::new(-ab.y, ab.x);
        let a1 = a + p.mul_s(thickness);
        let b1 = b + p.mul_s(thickness);
        top(&mut w, height, a, a1, b, b1);
        top(&mut w, 0.0, a, a1, b, b1);
        side(&mut w, height, a, b);
        side(&mut w, height, a1, b1);
    }
    w
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
    let obj = to_wavefront(0.001, 0.005, expand_element(relation, &osm.elements));
    println!("{}", obj.to_string());
}
