use std::collections::HashMap;
use std::iter;

pub trait Window {
    fn size(&self) -> usize;
    fn shape(&self) -> &[f32];

    fn apply(&self, data: &mut [f32]) {
        let shp = self.shape();
        assert_eq!(shp.len(), data.len());
        for (datum, factor) in data.iter_mut().zip(shp.iter()) {
            *datum *= factor;
        }
    }
}

pub struct GenericWindow(Vec<f32>);

impl Window for GenericWindow {
    fn size(&self) -> usize { self.0.len() }
    fn shape(&self) -> &[f32] { &self.0 }
}

fn new_rect(sz: usize) -> Box<dyn Window> {
    Box::new(GenericWindow(
            iter::repeat(1.0f32).take(sz).collect()
    ))
}

pub fn windows() -> HashMap<String, fn(usize) -> Box<dyn Window>> {
    let mut map: HashMap<String, fn(usize) -> Box<dyn Window>> = HashMap::new();
    map.insert("rect".into(), new_rect);
    map
}
