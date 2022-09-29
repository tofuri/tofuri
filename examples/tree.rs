use pea::{tree::Tree, util::timestamp};
fn main() {
    let mut tree = Tree::new();
    tree.insert([0x11; 32], [0x00; 32], timestamp());
    tree.insert([0x22; 32], [0x11; 32], timestamp());
    tree.insert([0x33; 32], [0x22; 32], timestamp());
    tree.insert([0x44; 32], [0x33; 32], timestamp());
    tree.insert([0x55; 32], [0x22; 32], timestamp());
    tree.insert([0x66; 32], [0x00; 32], timestamp());
    tree.insert([0x77; 32], [0x55; 32], timestamp() - 1);
    tree.sort_branches();
    println!("{:x?}", tree);
    println!("{:x?}", tree.main());
}
