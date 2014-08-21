extern crate osmmodels;
extern crate osmxml;
extern crate cgmath;
extern crate http;
extern crate serialize;
extern crate url;

use std::collections::HashMap;
use osmxml::{Osm, OsmElement, Relation, Way, Node};
use osmmodels::{Wavefront};
use cgmath::{Vector3, EuclideanVector, Vector};
use std::cmp::min;
use http::client::RequestWriter;
use http::method::Get;
use std::os;
use std::io::{File, Open, Write};
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

type V3 = Vector3<f64>;


fn top(w: &mut Wavefront, a: V3, a1: V3, b: V3, b1: V3) {
    w.add_vertex(a.x, a.y, a.z);
    w.add_vertex(a1.x, a1.y, a1.z);
    w.add_vertex(b1.x, b1.y, b1.z);
    w.add_vertex(b.x, b.y, b.z);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn bot(w: &mut Wavefront, a: V3, a1: V3, b: V3, b1: V3) {
    w.add_vertex(a.x, 0.0, a.z);
    w.add_vertex(a1.x, 0.0, a1.z);
    w.add_vertex(b1.x, 0.0, b1.z);
    w.add_vertex(b.x, 0.0, b.z);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn side(w: &mut Wavefront, a: V3, b: V3) {
    w.add_vertex(a.x, 0.0, a.z);
    w.add_vertex(a.x, a.y, a.z);
    w.add_vertex(b.x, b.y, b.z);
    w.add_vertex(b.x, 0.0, b.z);
    w.add_face(vec!(-1, -2, -3, -4));
}

fn to_wavefront(thickness: f64, ways: Vec<Vec<V3>>) -> Wavefront {
    let mut w = Wavefront::new();
    for coords in ways.iter() {
        let mut iter = coords.iter().zip(coords.iter().skip(1)).zip(coords.iter().skip(2));
        for ((&a, &b), &c) in iter {
            let ab = Vector3::new(b.x - a.x, 0.0, b.z - a.z).normalize();
            let bc = Vector3::new(c.x - b.x, 0.0, c.z - b.z).normalize();
            let p = Vector3::new(-ab.y, 0.0, ab.x);
            let p2 = Vector3::new(-bc.y, 0.0, bc.x);
            let a1 = a + p.mul_s(thickness);
            let b1 = b + p.mul_s(thickness);
            let c1 = c + p2.mul_s(thickness);
            top(&mut w, a, a1, b, b1);
            side(&mut w, a, b);
            side(&mut w, a1, b1);
            bot(&mut w, a, a1, b, b1);

            top(&mut w, b, b1, c, c1);
            side(&mut w, b, c);
            side(&mut w, b1, c1);
            bot(&mut w, b, b1, c, c1);
        }

        let &a = coords.iter().nth(coords.len() - 2).unwrap();
        let &b = coords.iter().last().unwrap();
        let ab = Vector3::new(b.x - a.x, 0.0, b.z - a.z).normalize();
        let p = Vector3::new(-ab.y, 0.0, ab.x);
        let a1 = a + p.mul_s(thickness);
        let b1 = b + p.mul_s(thickness);
        top(&mut w, a, a1, b, b1);
        side(&mut w, a, b);
        side(&mut w, a1, b1);
        bot(&mut w, a, a1, b, b1);

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

fn get_heights_request(api: &String, latlngs: &[(f64, f64)]) -> Vec<f64> {
    let mut heights = Vec::new();
    let s: Vec<String> = latlngs.iter()
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
    for chunk in latlngs.as_slice().chunks(100) {
        heights.push_all(get_heights_request(&api, chunk).as_slice());
    }
    heights
}

fn latlng_to_metres(lat: f64, lng: f64) -> (f64, f64) {
    // not technically true. Luckilly it shouldn't matter at the scale we deal with
    (lat * 111111.0, lng * 111111.0 * lat.to_radians().cos())
}

fn latlngs_to_coords(ways: Vec<Vec<(f64, f64)>>, size: int) -> (Vec<Vec<V3>>, f64) {
    let mut coords = Vec::new();
    let flat = ways.as_slice().concat_vec();
    let mtrs: Vec<(f64, f64)> = 
        flat.iter()
            .map(|&(lat, lng)| latlng_to_metres(lat, lng)).collect();
    let heights = get_heights(&flat);
    let (_, min_h) = scale(&heights, 5);
    let xs = mtrs.iter().map(|&(x, _)| x).collect();
    let ys = mtrs.iter().map(|&(_, y)| y).collect();
    let (sx, min_x) = scale(&xs, size);
    let (sy, min_y) = scale(&ys, size);
    let s = if sx < sy {sx} else {sy};
    let mut i = 0;
    for latlngs in ways.iter() {
        let mut way = Vec::new();
        for &(lat, lng) in latlngs.iter() {
            let (x, y) = latlng_to_metres(lat, lng);
            way.push(Vector3::new((x - min_x) * s,
                                  (heights[i] - min_h) * s,
                                  (y - min_y) * s));
            i += 1
        }
        coords.push(way);
    }
    (coords, s)
}

fn main() {
    let args = os::args();
    let ref osm_filename = args[1];
    let ref track_name = args[2];
    let ref out_filename = args[3];
    let path = &Path::new(osm_filename.clone());
    println!("Reading osm file");
    let osm = Osm::new(path).unwrap();
    let relation = osm.elements.values().filter(|e| {
        match **e {
            Relation{tags: ref ts, ..} => {
                ts.find(&"name".to_string()) == Some(track_name)
            }
            _ => false
        }
    }).next().expect(format!("Could not find relation with name {}", track_name).as_slice());
    println!("Finding nodes of the track");
    let latlngs = expand_relation(relation, &osm.elements);
    println!("Converting to model");
    let (coords, scale) = latlngs_to_coords(latlngs, 200);
    let obj = to_wavefront(14.0 * scale, coords);
    println!("Saving");
    let mut f = File::open_mode(&Path::new(out_filename.clone()), Open, Write);
    f.write_str(obj.to_string().as_slice()).unwrap();
}
