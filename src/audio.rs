/*
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    env,
    fs::File,
    io::BufReader,
    iter::{self, Map},
    path::PathBuf,
    slice,
    time::Duration,
};

use rodio::{buffer::SamplesBuffer, source, Sample, Source};


trait IteratorSourceExt: Sized + Source
where
    Self::Item: Sample,
{
    fn resample<F, U>(self, f: F) -> Box<dyn Source<Item = Self::Item> + Send + Sync>
    where
        Self::Item: Send + Sync,
        F: FnMut(iter::StepBy<slice::Iter<Self::Item>>) -> U,
        U: ExactSizeIterator,
        U::Item: Sample;
}

impl<T> IteratorSourceExt for T
where
    T: Sized + Source,
    T::Item: Sample,
{
    fn resample<F, U>(self, mut f: F) -> Box<dyn Source<Item = Self::Item> + Send + Sync>
    where
        Self::Item: Send + Sync,
        F: FnMut(iter::StepBy<slice::Iter<Self::Item>>) -> U,
        U: ExactSizeIterator,
        U::Item: Sample,
    {
        let mut max_chunk_size = MAX_RESAMPLE_CHUNK_SIZE * self.channels() as usize;

        let mut chunk_size = max_chunk_size;
        let _self = &mut self;
        let mut new_frame = true;
        let mut chunk = Vec::new();

        Box::new(source::from_iter(iter::repeat_with(|| {
            if new_frame {
                if let Some(frame_len) = _self.current_frame_len() {
                    chunk_size = max_chunk_size.min(frame_len * _self.channels() as usize);
                }

                new_frame = false;
            } else {
                if let Some(frame_len) = _self.current_frame_len() {
                    if frame_len < chunk_size {
                        new_frame = true;
                    }
                }
            };

            chunk.clear();
            chunk.extend(_self.take(chunk_size));

            let out: Vec<_> = (0.._self.channels())
                .map(|channel_idx| {
                    let out = f(chunk
                        .iter()
                        .dropping(channel_idx as usize)
                        .step_by(_self.channels() as usize));
                })
                .interleave()
                .collect();

            let new_sample_rate = if chunk.len() == out.len() {
                _self.sample_rate()
            } else {
                (_self.sample_rate() as f64 * (out.len() as f64 / chunk.len() as f64)) as u32
            };

            SamplesBuffer::new(self.channels(), new_sample_rate, chunk)
        })))
    }
}

const MAX_RESAMPLE_CHUNK_SIZE: usize = 1024 * 100;
*/
