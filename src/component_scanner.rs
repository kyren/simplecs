#[derive(Debug, Fail)]
pub enum ComponentSingletonError {
    #[fail(display = "no such singleton entry in scan")]
    Missing,
    #[fail(display = "multiple entries found in singleton query")]
    Multiple,
}

pub trait ComponentScanner {
    type Item;

    /// Keep moving the scanner forward in index order until the associated index is equal to or
    /// greater than the given index, then return the item and index of that entry, if it exists.
    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)>;

    fn iter(self) -> ComponentScannerIterator<Self>
    where
        Self: Sized,
    {
        ComponentScannerIterator(self)
    }

    /// If only a single entity should match, get the result out and error if there is not exactly
    /// one entry available.
    fn singleton(mut self) -> Result<Self::Item, ComponentSingletonError>
    where
        Self: Sized,
    {
        let (item, _) = self.scan(None).ok_or(ComponentSingletonError::Missing)?;
        if self.scan(None).is_some() {
            Err(ComponentSingletonError::Multiple)
        } else {
            Ok(item)
        }
    }

    /// Maps the given function over the items returned by scan
    fn map<R, F: Fn(Self::Item) -> R>(self, f: F) -> ComponentScannerMap<Self, F>
    where
        Self: Sized,
    {
        ComponentScannerMap {
            scanner: self,
            function: f,
        }
    }

    /// Converts this ComponentScanner into one that exists for all indexes and instead returns
    /// Some(Item) if it exists and None otherwise.
    fn opt(mut self) -> ComponentScannerOpt<Self, Self::Item>
    where
        Self: Sized,
    {
        let first = self.scan(None);
        ComponentScannerOpt {
            scanner: self,
            current: 0,
            next: first,
        }
    }

    /// Converts this ComponentScanner into one that only has values when the given scanner also
    /// has a value.
    fn limit<T>(
        self,
        and: T,
    ) -> ComponentScannerMap<
        ComponentScannerJoin<Self, T>,
        fn(<ComponentScannerJoin<Self, T> as ComponentScanner>::Item) -> Self::Item,
    >
    where
        Self: Sized,
        T: ComponentScanner,
    {
        fn fst<A, B>((i, _): (A, B)) -> A {
            i
        }

        ComponentScannerMap {
            scanner: ComponentScannerJoin(self, and),
            function: fst as fn((Self::Item, T::Item)) -> Self::Item,
        }
    }

    /// Converts this ComponentScanner into one that only has values when the given scanner does
    /// NOT have a value, inverse of limit.
    fn not<T>(self, mut not: T) -> ComponentScannerNot<Self, T>
    where
        Self: Sized,
        T: ComponentScanner,
    {
        let next_not = not.scan(None).map(|(_, ind)| ind);
        ComponentScannerNot {
            scanner: self,
            not_scanner: not,
            next_not: next_not,
        }
    }
}

impl<T: ComponentScanner + ?Sized> ComponentScanner for Box<T> {
    type Item = T::Item;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        (**self).scan(until)
    }
}

pub struct ComponentScannerIterator<T>(T);

impl<T> Iterator for ComponentScannerIterator<T>
where
    T: ComponentScanner,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.scan(None).map(|(i, _)| i)
    }
}

pub struct ComponentScannerMap<T, F> {
    scanner: T,
    function: F,
}

impl<T, F, R> ComponentScanner for ComponentScannerMap<T, F>
where
    T: ComponentScanner,
    F: Fn(T::Item) -> R,
{
    type Item = R;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        self.scanner
            .scan(until)
            .map(|(item, id)| ((self.function)(item), id))
    }
}

pub struct ComponentScannerNot<T1, T2> {
    scanner: T1,
    not_scanner: T2,
    next_not: Option<usize>,
}

impl<T1, T2, I> ComponentScanner for ComponentScannerNot<T1, T2>
where
    T1: ComponentScanner<Item = I>,
    T2: ComponentScanner,
{
    type Item = I;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        'begin: loop {
            match self.scanner.scan(until) {
                Some((item, index)) => loop {
                    match self.next_not {
                        Some(next_not) if next_not < index => {
                            self.next_not = self.not_scanner.scan(Some(index)).map(|(_, ind)| ind);
                        }
                        Some(next_not) if next_not == index => {
                            continue 'begin;
                        }
                        _ => {
                            return Some((item, index));
                        }
                    }
                },
                None => {
                    return None;
                }
            }
        }
    }
}

pub struct ComponentScannerOpt<T, I> {
    scanner: T,
    current: usize,
    next: Option<(I, usize)>,
}

impl<T, I> ComponentScanner for ComponentScannerOpt<T, I>
where
    T: ComponentScanner<Item = I>,
{
    type Item = Option<I>;

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        let until = until.unwrap_or(0);
        if self.current < until {
            self.current = until;
        }

        loop {
            if let Some((next_value, next_index)) = self.next.take() {
                if self.current > next_index {
                    self.next = self.scanner.scan(Some(self.current));
                    continue;
                }

                if self.current == next_index {
                    let val = Some((Some(next_value), self.current));
                    self.current += 1;
                    self.next = self.scanner.scan(None);
                    return val;
                } else {
                    let val = Some((None, self.current));
                    self.current += 1;
                    self.next = Some((next_value, next_index));
                    return val;
                }
            } else {
                let val = Some((None, self.current));
                self.current += 1;
                return val;
            }
        }
    }
}

pub struct ComponentScannerJoin<H, T>(H, T);

impl<H: ComponentScanner, T: ComponentScanner> ComponentScanner for ComponentScannerJoin<H, T> {
    type Item = (H::Item, T::Item);

    fn scan(&mut self, until: Option<usize>) -> Option<(Self::Item, usize)> {
        match (self.0.scan(until), self.1.scan(until)) {
            (Some((mut item1, mut id1)), Some((mut item2, mut id2))) => loop {
                if id1 < id2 {
                    if let Some(res) = self.0.scan(Some(id2)) {
                        item1 = res.0;
                        id1 = res.1;
                    } else {
                        return None;
                    }
                } else if id1 > id2 {
                    if let Some(res) = self.1.scan(Some(id1)) {
                        item2 = res.0;
                        id2 = res.1;
                    } else {
                        return None;
                    }
                } else {
                    return Some(((item1, item2), id1));
                }
            },
            _ => None,
        }
    }
}

pub trait ComponentScannerTuple {
    type JoinScanner: ComponentScanner;

    fn join(self) -> Self::JoinScanner;
}

macro_rules! impl_tuple {
    ($first:ident) => (
        impl<$first> ComponentScannerTuple for ($first,)
            where $first: ComponentScanner,
        {
            type JoinScanner = ComponentScannerMap<$first, fn($first::Item) -> ($first::Item,)>;

            fn join(self) -> Self::JoinScanner {
                self.0.map(|item| (item,))
            }
        }
    );

    ($first:ident $($rest:ident)+) => (
        impl<$first, $($rest,)*> ComponentScannerTuple for ($first, $($rest,)*)
            where $first: ComponentScanner,
                  $($rest: ComponentScanner,)*
        {
            type JoinScanner = ComponentScannerMap<
                ComponentScannerJoin<$first, <($($rest,)*) as ComponentScannerTuple>::JoinScanner>,
                fn(($first::Item,
                    <<($($rest,)*) as ComponentScannerTuple>::JoinScanner as ComponentScanner>::Item
                )) -> ($first::Item, $($rest::Item,)*)
            >;

            #[allow(non_snake_case)]
            fn join(self) -> Self::JoinScanner {
                let ($first, $($rest,)*) = self;
                ComponentScannerJoin($first, ($($rest,)*).join())
                    .map(|($first, ($($rest,)*))| ($first, $($rest,)*))
            }
        }
    );
}

pub fn component_scan_join<T: ComponentScannerTuple>(t: T) -> T::JoinScanner {
    t.join()
}

impl_tuple!{A}
impl_tuple!{A B}
impl_tuple!{A B C}
impl_tuple!{A B C D}
impl_tuple!{A B C D E}
impl_tuple!{A B C D E F}
impl_tuple!{A B C D E F G}
impl_tuple!{A B C D E F G H}
impl_tuple!{A B C D E F G H I}
impl_tuple!{A B C D E F G H I J}
impl_tuple!{A B C D E F G H I J K}
impl_tuple!{A B C D E F G H I J K L}
impl_tuple!{A B C D E F G H I J K L M}
impl_tuple!{A B C D E F G H I J K L M N}
impl_tuple!{A B C D E F G H I J K L M N O}
impl_tuple!{A B C D E F G H I J K L M N O P}
