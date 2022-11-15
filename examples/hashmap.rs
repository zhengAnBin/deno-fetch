use std::collections::BTreeMap;

fn main() {
    let mut tree_map = BTreeMap::new();
    tree_map.insert(1, 2);
    println!("{:?}", tree_map.get(&1));
}
