use std::io::Writer;

#[deriving(Show)]
pub enum WavefrontValue {
    Vertex(f32, f32, f32),
    Face(Vec<int>)
}

impl WavefrontValue {
    pub fn to_string(&self) -> String {
        match *self {
            Vertex(x, y, z) => format!("v {} {} {}", x, y, z),
            Face(ref locs) =>  {
                let locs: Vec<String> = locs.iter().map(|v| v.to_string()).collect();
                format!("f {}", locs.connect(" "))
            }
        }
    }
}

pub struct Wavefront {
    pub values: Vec<WavefrontValue>
}

impl Wavefront {
    pub fn new() -> Wavefront {
        Wavefront {values: Vec::new()}
    }

    pub fn to_string(&self) {
        let vals: Vec<String> = self.values.iter().map(|v| v.to_string()).collect();
        vals.connect("\n");
    }

    pub fn add_vertex(&mut self, x: f32, y: f32, z: f32) {
        self.values.push(Vertex(x, y, z));
    }

    pub fn add_face(&mut self, vec: Vec<int>) {
        self.values.push(Face(vec));
    }
}
