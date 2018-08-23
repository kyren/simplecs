use component::*;
use component_scanner::*;
use dense_component::*;
use sparse_component::*;

#[test]
fn test_join() {
    fn go<S>()
    where
        S: for<'a> ComponentStorage<'a, Component = i32>,
    {
        let mut compa = S::default();
        let mut compb = S::default();

        compa.insert(1, 1);
        compa.insert(2, 2);
        compa.insert(3, 3);
        compa.insert(5, 5);
        compa.insert(6, 6);
        compa.insert(7, 7);
        compa.insert(9, 9);

        compb.insert(2, 2);
        compb.insert(4, 4);
        compb.insert(6, 6);
        compb.insert(9, 9);

        let values = component_scan_join((compa.scan(), compb.scan()))
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(values, vec![(&2, &2), (&6, &6), (&9, &9)]);

        let mut value_scan = component_scan_join((compa.scan(), compb.scan()));
        assert_eq!(value_scan.scan(None), Some(((&2, &2), 2)));
        assert_eq!(value_scan.scan(Some(7)), Some(((&9, &9), 9)));
    }

    go::<SparseComponentStorage<i32>>();
    go::<DenseComponentStorage<i32>>();
}

#[test]
fn test_opt() {
    fn go<S>()
    where
        S: for<'a> ComponentStorage<'a, Component = i32>,
    {
        let mut comp = S::default();
        comp.insert(1, 1);
        comp.insert(3, 3);
        comp.insert(5, 5);
        comp.insert(6, 6);

        let values = comp.scan().opt().iter().take(8).collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                None,
                Some(&1),
                None,
                Some(&3),
                None,
                Some(&5),
                Some(&6),
                None,
            ]
        );

        let mut value_scan = comp.scan().opt();
        assert_eq!(value_scan.scan(Some(2)), Some((None, 2)));
        assert_eq!(value_scan.scan(Some(5)), Some((Some(&5), 5)));
        assert_eq!(value_scan.scan(None), Some((Some(&6), 6)));
        assert_eq!(value_scan.scan(None), Some((None, 7)));
    }

    go::<SparseComponentStorage<i32>>();
    go::<DenseComponentStorage<i32>>();
}

#[test]
fn test_limit() {
    fn go<S>()
    where
        S: for<'a> ComponentStorage<'a, Component = i32>,
    {
        let mut compa = S::default();
        let mut compb = S::default();

        compa.insert(1, 1);
        compa.insert(2, 2);
        compa.insert(3, 3);
        compa.insert(5, 5);
        compa.insert(6, 6);
        compa.insert(7, 7);
        compa.insert(9, 9);

        compb.insert(2, 2);
        compb.insert(4, 4);
        compb.insert(6, 6);
        compb.insert(9, 9);

        let values = compa.scan().limit(compb.scan()).iter().collect::<Vec<_>>();
        assert_eq!(values, vec![&2, &6, &9]);

        let mut value_scan = compa.scan().limit(compb.scan());
        assert_eq!(value_scan.scan(Some(3)), Some((&6, 6)));
        assert_eq!(value_scan.scan(None), Some((&9, 9)));
    }

    go::<SparseComponentStorage<i32>>();
    go::<DenseComponentStorage<i32>>();
}

#[test]
fn test_not() {
    fn go<S>()
    where
        S: for<'a> ComponentStorage<'a, Component = i32>,
    {
        let mut compa = S::default();
        let mut compb = S::default();

        compa.insert(1, 1);
        compa.insert(2, 2);
        compa.insert(3, 3);
        compa.insert(5, 5);
        compa.insert(6, 6);
        compa.insert(7, 7);
        compa.insert(9, 9);

        compb.insert(2, 2);
        compb.insert(4, 4);
        compb.insert(6, 6);

        let values = compa.scan().not(compb.scan()).iter().collect::<Vec<_>>();
        assert_eq!(values, vec![&1, &3, &5, &7, &9]);

        let mut value_scan = compa.scan().not(compb.scan());

        assert_eq!(value_scan.scan(Some(2)), Some((&3, 3)));
        assert_eq!(value_scan.scan(None), Some((&5, 5)));
        assert_eq!(value_scan.scan(Some(8)), Some((&9, 9)));
    }

    go::<SparseComponentStorage<i32>>();
    go::<DenseComponentStorage<i32>>();
}

// Make sure that Box<ComponentScanner> works as expected and can be used for dynamically
// configured scanning.
#[test]
fn test_dynamic() {
    fn go<S>()
    where
        S: for<'a> ComponentStorage<'a, Component = i32>,
    {
        let mut compa = S::default();
        let mut compb = S::default();

        compa.insert(1, 1);
        compa.insert(2, 2);
        compa.insert(3, 3);
        compa.insert(5, 5);
        compa.insert(6, 6);
        compa.insert(7, 7);

        compb.insert(2, 2);
        compb.insert(4, 4);
        compb.insert(6, 6);

        let mut value_scan: Box<ComponentScanner<Item = _>> = Box::new(compa.scan());
        value_scan = Box::new(value_scan.not(compb.scan()));

        assert_eq!(value_scan.scan(Some(2)), Some((&3, 3)));
        assert_eq!(value_scan.scan(None), Some((&5, 5)));
        assert_eq!(value_scan.scan(Some(6)), Some((&7, 7)));
    }

    go::<SparseComponentStorage<i32>>();
    go::<DenseComponentStorage<i32>>();
}
