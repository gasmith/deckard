use super::*;

#[test]
fn test_preorder() {
    // d -- f -- b -- c
    //        `- g -- a
    //             `- e
    let mut builder = Tree::builder();
    let ids: Vec<_> = "abcdefg".chars().map(|c| builder.insert(c)).collect();
    for (c, p) in [(0, 6), (1, 5), (2, 1), (4, 6), (5, 3), (6, 5)] {
        builder.set_parent(ids[c], ids[p]);
    }
    let tree = builder.build();
    let nodes: Vec<_> = tree
        .preorder()
        .map(|n| (*n.data, n.sibling, n.last_sibling, n.leaf))
        .collect();
    assert_eq!(
        nodes,
        vec![
            ('d', false, false, false),
            ('f', false, false, false),
            ('b', true, false, false),
            ('c', false, false, true),
            ('g', true, true, false),
            ('a', true, false, true),
            ('e', true, true, true),
        ]
    );
}
