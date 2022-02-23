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
    CommonPrefix,
    Object,
}

#[derive(Clone, Debug, PartialEq)]
pub enum S3Item {
    Pop, //상위 디렉토리를 가리키는 객체
    Bucket(BucketWithLocation),
    CommonPrefix(CommonPrefix),
    Object(Object),
}

fn last_component(key_or_prefix: &str) -> String {
    let last_component_impl =
        |str: &str| str.split("/").into_iter().last().unwrap_or("").to_owned();

    // case of prefix
    if let Some(str) = key_or_prefix.strip_suffix("/") {
        last_component_impl(str) + "/"
    } else {
        last_component_impl(key_or_prefix)
    }
}

impl S3Item {
    pub fn is_matched(&self, search_text: &str) -> bool {
        // TODO: use regex
        match self {
            S3Item::Pop => false,
            S3Item::Bucket(b) => b.bucket.name().unwrap_or_default().contains(search_text),
            S3Item::CommonPrefix(c) => {
                last_component(c.prefix().unwrap_or("")).contains(search_text)
            }
            S3Item::Object(k) => last_component(k.key().unwrap_or("")).contains(search_text),
        }
    }

    pub fn get_type(&self) -> S3ItemType {
        match self {
            S3Item::Pop => S3ItemType::Pop,
            S3Item::Bucket(_) => S3ItemType::Bucket,
            S3Item::CommonPrefix(_) => S3ItemType::CommonPrefix,
            S3Item::Object(_) => S3ItemType::Object,
        }
    }
    pub fn as_row(&self) -> (String, String, String) {
        match self {
            S3Item::CommonPrefix(d) => (
                "PRE".to_owned(),
                String::default(),
                last_component(d.prefix().unwrap_or("")),
            ),

            S3Item::Object(k) => (
                k.last_modified()
                    .map(|m| m.fmt(Format::DateTime).unwrap_or_default())
                    .unwrap_or(String::default()),
                ByteSize(k.size() as u64).to_string_as(true),
                last_component(k.key().unwrap_or("")),
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
        S3Item::CommonPrefix(common_prefix)
    }
}

impl From<Object> for S3Item {
    fn from(object: Object) -> Self {
        S3Item::Object(object)
    }
}
