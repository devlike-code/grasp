#[cfg(test)]
mod query_utility_tests {
    use crate::querying::querying::{take_arrows, targets_from, tiles, MosaicCollage};

    use itertools::Itertools;
    use mosaic::internals::{void, Mosaic, MosaicCRUD, MosaicIO};
    #[test]
    fn collage_test() {
        let mosaic = Mosaic::new();
        let t = mosaic.new_object("void", void());
        let u = mosaic.new_object("void", void());
        let v = mosaic.new_object("void", void());
        mosaic.new_arrow(&t, &u, "void", void());
        mosaic.new_arrow(&t, &v, "void", void());
        let mq = targets_from(take_arrows(tiles()));
        let mut result = mosaic.apply_collage(&mq, None).collect_vec();
        result.sort();
        assert_eq!(vec![u.clone(), v.clone()], result);
    }

    #[test]
    fn collage_test_limited_to_some_tiles() {
        let mosaic = Mosaic::new();
        let t = mosaic.new_object("void", void());
        let u = mosaic.new_object("void", void());
        let v = mosaic.new_object("void", void());
        mosaic.new_arrow(&t, &u, "void", void());
        mosaic.new_arrow(&t, &v, "void", void());
        let mq = targets_from(take_arrows(tiles()));
        let selection = vec![t.clone(), u.clone()];
        let mut result = mosaic.apply_collage(&mq, Some(selection)).collect_vec();
        result.sort();
        assert_eq!(vec![u.clone()], result);
    }
}
