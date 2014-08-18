extern crate track_visulisation;
extern crate osmxml;
extern crate cgmath;
extern crate http;
extern crate serialize;
extern crate url;

use std::collections::HashMap;
use osmxml::{Osm, OsmElement, Relation, Way, Node};
use track_visulisation::{Wavefront};
use cgmath::{Vector3, EuclideanVector, Vector};
use std::cmp::min;
use http::client::RequestWriter;
use http::method::Get;
use std::os;
use url::Url;
use serialize::json;
use std::str;

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

type Vec3 = Vector3<f64>;


fn top(w: &mut Wavefront, a: Vec3, a1: Vec3, b: Vec3, b1: Vec3) {
    w.add_vertex(a.x, a.y, a.z);
    w.add_vertex(a1.x, a1.y, a1.z);
    w.add_vertex(b1.x, b1.y, b1.z);
    w.add_vertex(b.x, b.y, b.z);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn bot(w: &mut Wavefront, a: Vec3, a1: Vec3, b: Vec3, b1: Vec3) {
    w.add_vertex(a.x, 0.0, a.z);
    w.add_vertex(a1.x, 0.0, a1.z);
    w.add_vertex(b1.x, 0.0, b1.z);
    w.add_vertex(b.x, 0.0, b.z);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn side(w: &mut Wavefront, a: Vec3, b: Vec3) {
    w.add_vertex(a.x, 0.0, a.z);
    w.add_vertex(a.x, a.y, a.z);
    w.add_vertex(b.x, b.y, b.z);
    w.add_vertex(b.x, 0.0, b.z);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn to_wavefront(thickness: f64, ways: Vec<Vec<Vec3>>) -> Wavefront {
    let mut w = Wavefront::new();
    for coords in ways.iter() {
        let mut iter = coords.iter().zip(coords.iter().skip(1));
        for (&a, &b) in iter {
            let ab = Vector3::new(b.x - a.x, 0.0, b.z - a.z).normalize();
            let p = Vector3::new(-ab.y, 0.0, ab.x);
            let a1 = a + p.mul_s(thickness);
            let b1 = b + p.mul_s(thickness);
            top(&mut w, a, a1, b, b1);
            side(&mut w, a, b);
            side(&mut w, a1, b1);
            bot(&mut w, a, a1, b, b1);
        }
    }
    w
}

fn scale(points: &Vec<f64>, size: int) -> (f64, f64) {
    // see http://www.reddit.com/r/rust/comments/29kia3/no_ord_for_f32/
    let mut points = points.clone();
    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
    let min = points[0];
    let max = points.last().unwrap();
    ((size as f64) / (max - min), min)
}

fn get_heights_iter<'a, I: Iterator<&'a (f64, f64)>>(api: &String, latlngs: I) -> Vec<f64> {
    let mut heights = Vec::new();
    let s: Vec<String> = latlngs.take(100)
                                .map(|&(lat,lng)| format!("{},{}", lat, lng))
                                .collect();
    let url = Url::parse(format!(
        "https://maps.googleapis.com/maps/api/elevation/json?key={}&locations={}",
        api,
        s.connect("|")).as_slice()).unwrap();
    let request: RequestWriter = RequestWriter::new(Get, url).unwrap();
    let mut response = match request.read_response() {
        Ok(r) => r,
        Err(e) => fail!("Could not read response")
    };
    let body = response.read_to_end().unwrap();
    let s = str::from_utf8(body.as_slice()).expect("body from_utf8");
    let json: json::Json = json::from_str(s).unwrap();
    for res in json.find(&"results".to_string()).unwrap().as_list().unwrap().iter() {
        heights.push(res.find(&"elevation".to_string()).unwrap().as_number().unwrap());
    }
    heights
}

fn get_heights(latlngs: &Vec<(f64, f64)>) -> Vec<f64> {
    let api: String = os::getenv("GAPI").expect("Please set GAPI");
    let mut heights = Vec::new();
    let mut i = 0;
    loop {
        let iter = latlngs.iter().skip(i).take(100);
        if i > latlngs.len() {
            break;
        }
        heights.push_all(get_heights_iter(&api, iter).as_slice());
        i += 100;
    }
    heights
}

fn latlngs_to_coords(ways: Vec<Vec<(f64, f64)>>, size: int) -> Vec<Vec<Vec3>> {
    let mut coords = Vec::new();
    let flat = ways.as_slice().concat_vec();
    let heights = get_heights(&flat);
    let (sh, min_h) = scale(&heights, 5);
    let lats = flat.iter().map(|&(x, _)| x).collect();
    let lngs = flat.iter().map(|&(_, y)| y).collect();
    let (sx, min_x) = scale(&lats, size);
    let (sy, min_y) = scale(&lngs, size);
    let s = if sx < sy {sx} else {sy};
    let mut i = 0;
    for latlngs in ways.iter() {
        let mut way = Vec::new();
        for &(lat, lng) in latlngs.iter() {
            way.push(Vector3::new((lat - min_x) * s,
                                  (heights[i] - min_h) * sh + 0.1,
                                  (lng - min_y) * s));
            i += 1
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
    let obj = to_wavefront(0.5, coords);
    println!("{}", obj.to_string());
}
