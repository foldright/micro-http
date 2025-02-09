#[derive(Debug, Copy, Clone)]
pub struct TestCase {
    name: &'static str,
    group: TestGroup,
    file: TestFile,
}

impl TestCase {
    pub fn new(name: &'static str, group: TestGroup, file: TestFile) -> Self {
        Self { name, group, file }
    }

    pub fn small(name: &'static str, file: TestFile) -> Self {
        Self::new(name, TestGroup::Small, file)
    }

    pub fn normal(name: &'static str, file: TestFile) -> Self {
        Self::new(name, TestGroup::Normal, file)
    }

    pub fn large(name: &'static str, file: TestFile) -> Self {
        Self::new(name, TestGroup::Large, file)
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn group(&self) -> TestGroup {
        self.group
    }

    pub fn file(&self) -> &TestFile {
        &self.file
    }

    pub fn file_name(&self) -> &'static str {
        self.file().file_name
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TestFile {
    file_name: &'static str,
    content: &'static str,
}

impl TestFile {
    pub const fn new(file_name: &'static str, content: &'static str) -> Self {
        Self { file_name, content }
    }

    pub fn content(&self) -> &'static str {
        self.content
    }

    pub fn file_name(&self) -> &'static str {
        self.file_name
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TestGroup {
    Small,
    Normal,
    Large,
}
