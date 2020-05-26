use render::{Point2, Point3, Line};
use crate::Box2D;

pub fn rocket() -> Box<[Line]> {
    let color = Point3::new(1.0, 0.0, 0.0);
    let verices = [
        Point2::new(-5.0, 0.0),
        Point2::new(-10.0, 5.0),
        Point2::new(10.0, 0.0),
        Point2::new(-10.0, -5.0),
    ];
    Box::new([
        (verices[0], verices[1], color),
        (verices[1], verices[2], color),
        (verices[2], verices[3], color),
        (verices[3], verices[0], color),
    ])
}

pub fn bullet() -> Box<[Line]> {
    let color = Point3::new(0.0, 1.0, 1.0);
    let verices = [
        Point2::new(-2.0, 0.0),
        Point2::new(0.0, 2.0),
        Point2::new(2.0, 0.0),
        Point2::new(0.0, -2.0),
    ];
    Box::new([
        (verices[0], verices[1], color),
        (verices[1], verices[2], color),
        (verices[2], verices[3], color),
        (verices[3], verices[0], color),
    ])
}

pub fn rectangle(b: &Box2D, color: Point3<f32>) -> Box<[Line]> {
    let verices = [
        Point2::new(b.min.x, b.min.y),
        Point2::new(b.max.y, b.min.y),
        Point2::new(b.max.y, b.max.y),
        Point2::new(b.min.y, b.max.y),
    ];
    Box::new([
        (verices[0], verices[1], color),
        (verices[1], verices[2], color),
        (verices[2], verices[3], color),
        (verices[3], verices[0], color),
    ])
}
