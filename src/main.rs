extern crate track_visulisation;
extern crate osmxml;
extern crate cgmath;

use std::collections::HashMap;
use osmxml::{Osm, OsmElement, Relation, Way, Node};
use track_visulisation::{Wavefront};
use cgmath::{Vector2, EuclideanVector, Vector};
use std::cmp::min;

fn expand_node(elem: &OsmElement) -> (f64, f64) {
    match *elem {
        Node {lat: lat, lng: lng, ..} => (lat, lng),
        _ => fail!("expand_node must be passed a node")
    }
}

fn expand_way(elem: &OsmElement, elements: &HashMap<int, OsmElement>) -> Vec<(f64, f64)> {
    let refs = match *elem {
        Way {nodes: ref ns, ..} => ns,
        _ => fail!("expand_way must be passed a way")
    };
    let mut latlngs = Vec::new();
    for r in refs.iter() {
        match elements.find(r) {
            Some(e@&Node{..}) => latlngs.push(expand_node(e)),
            None => fail!("Could not find element with id {}", r),
            _ => ()
        }
    }
    latlngs
}

fn expand_relation(elem: &OsmElement, elements: &HashMap<int, OsmElement>) -> Vec<Vec<(f64, f64)>> {
    let refs = match *elem {
        Relation {members: ref m, ..} => m,
        _ => fail!("expand_relation must be passed a relation"),
    };
    let mut ways = Vec::new(); 
    for r in refs.iter() {
        match elements.find(r) {
            Some(e@&Way{..}) => ways.push(expand_way(e, elements)),
            None => fail!("Could not find element with id {}", r),
            _ => (),
        };
    }
    ways
}

type Vec2 = Vector2<f64>;

fn top(w: &mut Wavefront, height: f64, a: Vec2, a1: Vec2, b: Vec2, b1: Vec2) {
    w.add_vertex(a.x, height, a.y);
    w.add_vertex(a1.x, height, a1.y);
    w.add_vertex(b1.x, height, b1.y);
    w.add_vertex(b.x, height, b.y);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn side(w: &mut Wavefront, height: f64, a: Vec2, b: Vec2) {
    w.add_vertex(a.x, 0.0, a.y);
    w.add_vertex(a.x, height, a.y);
    w.add_vertex(b.x, height, b.y);
    w.add_vertex(b.x, 0.0, b.y);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn to_wavefront(thickness: f64, height: f64, ways: Vec<Vec<Vec2>>) -> Wavefront {
    let mut w = Wavefront::new();
    for latlngs in ways.iter() {
        let mut iter = latlngs.iter().zip(latlngs.iter().skip(1));
        for (&a, &b) in iter {
            let ab = Vector2::new(b.x - a.x, b.y - a.y).normalize();
            let p = Vector2::new(-ab.y, ab.x);
            let a1 = a + p.mul_s(thickness);
            let b1 = b + p.mul_s(thickness);
            top(&mut w, height, a, a1, b, b1);
            side(&mut w, height, a, b);
            side(&mut w, height, a1, b1);
            top(&mut w, 0.0, a, a1, b, b1);
        }
    }
    w
}

fn scale(points: Vec<f64>, size: int) -> (f64, f64) {
    // see http://www.reddit.com/r/rust/comments/29kia3/no_ord_for_f32/
    let mut points = points;
    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
    let min = points[0];
    let max = points.last().unwrap();
    ((size as f64) / (max - min), min)
}

fn latlngs_to_coords(latlngs: Vec<Vec<(f64, f64)>>, size: int) -> Vec<Vec<Vec2>> {
    let mut coords = Vec::new();
    let flat = latlngs.as_slice().concat_vec();
    let lats = flat.iter().map(|&(x, _)| x).collect();
    let lngs = flat.iter().map(|&(_, y)| y).collect();
    let (sx, min_x) = scale(lats, size);
    let (sy, min_y) = scale(lngs, size);
    let s = if sx < sy {sx} else {sy};
    for ll in latlngs.iter() {
        let mut way = Vec::new();
        for &(lat, lng) in ll.iter() {
            way.push(Vector2::new((lat - min_x) * s, (lng - min_y) * s));
        }
        coords.push(way);
    }
    coords
}

fn main() {
    let path = &Path::new("spa.osm");
    let osm = Osm::new(path).unwrap();
    let track = "Ciruit de Spa Francorchamps".to_string();
    let relation = osm.elements.values().filter(|e| {
        match **e {
            Relation{tags: ref ts, ..} => {
                ts.find(&"name".to_string()) == Some(&track)
            }
            _ => false
        }
    }).next().unwrap();
    let latlngs = expand_relation(relation, &osm.elements);
    let coords = latlngs_to_coords(latlngs, 200);
    let obj = to_wavefront(0.5, 2.0, coords);
    println!("{}", obj.to_string());
}
