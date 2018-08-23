use component_scanner::ComponentScanner;

pub trait Component: 'static + Send + Sync + Sized + Clone {
    type Storage: for<'a> ComponentStorage<'a, Component = Self>;
}

pub trait ComponentStorage<'a>: 'static + Send + Sync + Default + Clone {
    type Component: 'static + Send + Sync + Sized + Clone;
    type Scan: ComponentScanner<Item = &'a Self::Component>;
    type ScanMut: ComponentScanner<Item = &'a mut Self::Component>;

    fn get(&self, index: usize) -> Option<&Self::Component>;
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Component>;

    fn insert(&mut self, index: usize, component: Self::Component) -> Option<Self::Component>;
    fn remove(&mut self, index: usize) -> Option<Self::Component>;

    fn scan(&'a self) -> Self::Scan;
    fn scan_mut(&'a mut self) -> Self::ScanMut;
}
