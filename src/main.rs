mod chunks;
mod util;

use chunks::{QOI_CHUNK, OP_DIFF, OP_INDEX, OP_RGB, OP_LUMA, OP_RGBA, OP_RUN};

fn main() {
    let buffer = std::fs::read("./qoi_test_images/dice.qoi").unwrap();

    let mut peekable = buffer.iter().skip(14).map(|&b| b).peekable();
    loop {
        if OP_DIFF::try_decode(&mut peekable).map(|chunk| {println!("{:?}", chunk); ()}).is_some() {
            continue;
        }
        if OP_INDEX::try_decode(&mut peekable).map(|chunk| {println!("{:?}", chunk); ()}).is_some() {
            continue;
        }
        if OP_LUMA::try_decode(&mut peekable).map(|chunk| {println!("{:?}", chunk); ()}).is_some() {
            continue;
        }
        if OP_RGBA::try_decode(&mut peekable).map(|chunk| {println!("{:?}", chunk); ()}).is_some() {
            continue;
        }
        if OP_RGB::try_decode(&mut peekable).map(|chunk| {println!("{:?}", chunk); ()}).is_some() {
            continue;
        }
        if OP_RUN::try_decode(&mut peekable).map(|chunk| {println!("{:?}", chunk); ()}).is_some() {
            continue;
        }
        break;
    }
}