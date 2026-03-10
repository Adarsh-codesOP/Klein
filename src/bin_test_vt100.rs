fn main() { let mut p = vt100::Parser::new(24, 80, 1000); p.process(b"hello\nworld"); println!("{:?}", p.screen().cursor_position()); }
