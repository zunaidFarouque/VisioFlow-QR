use image::GrayImage;

use crate::error::{Result, VisioFlowError};
use crate::traits::PayloadDecoder;

pub struct RqrrDecoder;

impl PayloadDecoder for RqrrDecoder {
    fn decode(&self, image: &GrayImage) -> Result<Vec<String>> {
        let mut prepared = rqrr::PreparedImage::prepare(image.clone());
        let grids = prepared.detect_grids();

        let mut payloads = Vec::new();
        for grid in grids {
            match grid.decode() {
                Ok((_, content)) => payloads.push(content),
                Err(_) => continue,
            }
        }

        if payloads.is_empty() {
            return Err(VisioFlowError::NoPayloads);
        }

        Ok(payloads)
    }
}
