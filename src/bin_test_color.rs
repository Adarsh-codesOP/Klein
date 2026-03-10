fn main() { let c = vt100::Color::Idx(0); match c { vt100::Color::Idx(_) => {}, vt100::Color::Rgb(_,_,_) => {}, _ => {} } }
