/*!
**Multi Channel Signed Distance Field renderer library.**

It allows you to create multi channel signed distance fields rendered
to the memory. Currently rendering from fonts is supported as well.
*/

#![warn(missing_docs)]

extern crate cgmath;
extern crate rusttype;

pub mod font;
pub mod geometry;
pub mod math;
pub mod renderer;
pub mod shape;
pub mod texture;
