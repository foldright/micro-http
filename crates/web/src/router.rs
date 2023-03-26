use crate::filter::Filter;
use crate::handler::RequestHandler;

pub fn resource() -> ResourceBuilder {
    ResourceBuilder::new()
}

pub struct Resource {
    inner: Vec<ResourceItem>,
}

impl Resource {
    pub fn resource_items_ref(&self) -> &[ResourceItem] {
        self.inner.as_slice()
    }
}

impl ResourceBuilder {
    fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn when<F: Filter + Send + Sync + 'static>(self, f: F) -> ResourceItemBuilder {
        ResourceItemBuilder::new(self).when(f)
    }

    fn item(mut self, item: ResourceItem) -> ResourceBuilder {
        self.inner.push(item);
        self
    }

    pub fn build(self) -> Resource {
        Resource { inner: self.inner }
    }
}

pub struct ResourceItem {
    filter: Option<Box<dyn Filter + Send + Sync>>,
    handler: Box<dyn RequestHandler>,
}

impl ResourceItem {
    pub fn filter_ref(&self) -> Option<&(dyn Filter + Send + Sync)> {
        self.filter.as_ref().map(|x| x.as_ref())
    }

    pub fn handler_ref(&self) -> &dyn RequestHandler {
        self.handler.as_ref()
    }
}

pub struct ResourceBuilder {
    inner: Vec<ResourceItem>,
}

pub struct ResourceItemBuilder {
    resource_builder: ResourceBuilder,
    filter: Option<Box<dyn Filter + Send + Sync>>,
    handler: Option<Box<dyn RequestHandler>>,
}

impl ResourceItemBuilder {
    fn new(resource_builder: ResourceBuilder) -> Self {
        Self { resource_builder, filter: None, handler: None }
    }

    fn when<F: Filter + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.filter = Some(Box::new(f));
        self
    }

    pub fn to<H: RequestHandler + 'static>(self, h: H) -> ResourceBuilder {
        let item = ResourceItem { filter: self.filter, handler: Box::new(h) };
        self.resource_builder.item(item)
    }
}
