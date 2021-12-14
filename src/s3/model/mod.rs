use aws_sdk_s3::model::{CommonPrefix, Object};
use aws_smithy_types::date_time::Format;
use bytesize::ByteSize;
use strum_macros::EnumIter;

use super::*;

use client::BucketWithLocation;

#[derive(Debug, PartialEq, EnumIter)]
pub enum S3ItemType {
    Pop,
    Bucket,
    Directory,
    Key,
}

#[derive(Clone, Debug, PartialEq)]
pub enum S3Item {
    Pop, //상위 디렉토리를 가리키는 객체
    Bucket(BucketWithLocation),
    Directory(CommonPrefix),
    Key(Object),
}

impl S3Item {
    pub fn get_type(&self) -> S3ItemType {
        match self {
            S3Item::Pop => S3ItemType::Pop,
            S3Item::Bucket(_) => S3ItemType::Bucket,
            S3Item::Directory(_) => S3ItemType::Directory,
            S3Item::Key(_) => S3ItemType::Key,
        }
    }
    pub fn as_row(&self) -> (String, String, String) {
        match self {
            S3Item::Directory(d) => (
                "PRE".to_owned(),
                String::default(),
                d.prefix().unwrap_or("").to_owned(),
            ),

            S3Item::Key(k) => (
                k.last_modified()
                    .map(|m| m.fmt(Format::DateTime).unwrap_or_default())
                    .unwrap_or(String::default()),
                ByteSize(k.size() as u64).to_string_as(true),
                k.key().unwrap_or("").to_owned(),
            ),
            S3Item::Bucket(b) => {
                let location = {
                    let location = b.location.as_str().to_owned();
                    if location.len() > 0 {
                        location
                    } else {
                        "unknown".to_owned()
                    }
                };
                let bucket_name = b.bucket.name().unwrap_or("").to_owned();
                (String::default(), location, bucket_name)
            }
            S3Item::Pop => ("..".to_owned(), String::default(), String::default()),
        }
    }
}

impl From<BucketWithLocation> for S3Item {
    fn from(bucket: BucketWithLocation) -> Self {
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
