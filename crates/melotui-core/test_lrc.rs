use lrc::Lyrics;
fn main() {
    let l = " [00:12.00] Line\n[00:14.00] Line 2".parse::<Lyrics>().unwrap();
    let lyrics = l.get_lyrics();
    println!("{:?}", lyrics.len());
    let x = &lyrics[0];
    println!("{:?}", x);
}
