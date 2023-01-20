use pea_tree::Tree;
fn main() {
    let mut tree = Tree::new();
    tree.insert([0x11; 32], [0x00; 32], 1);
    tree.insert([0x22; 32], [0x11; 32], 1);
    tree.insert([0x33; 32], [0x22; 32], 1);
    tree.insert([0x44; 32], [0x33; 32], 1);
    tree.insert([0x55; 32], [0x22; 32], 1);
    tree.insert([0x66; 32], [0x00; 32], 1);
    tree.insert([0x77; 32], [0x55; 32], 0);
    tree.sort_branches();
    println!("{tree:x?}");
    println!("{:x?}", tree.main());
}
