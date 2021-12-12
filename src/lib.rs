use aws_sdk_s3::model::{Bucket, CommonPrefix, Object};

pub mod controller;
pub mod frontend;
pub mod s3;
pub mod view_model;

#[derive(Clone, Debug, PartialEq)]
pub enum S3Item {
    Pop, //상위 디렉토리를 가리키는 객체
    Bucket(Bucket),
    Directory(CommonPrefix),
    Key(Object),
}

impl From<Bucket> for S3Item {
    fn from(bucket: Bucket) -> Self {
        S3Item::Bucket(bucket)
    }
}

impl From<CommonPrefix> for S3Item {
    fn from(common_prefix: CommonPrefix) -> Self {
        S3Item::Directory(common_prefix)
    }
}

impl From<Object> for S3Item {
    fn from(object: Object) -> Self {
        S3Item::Key(object)
    }
}
