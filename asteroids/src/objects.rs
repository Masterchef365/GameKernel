use crate::Box2D;
use render::{Line, Point2, Point3};

fn cannonicalize(points: &[Point2<f32>], color: Point3<f32>) -> Box<[Line]> {
    let mut lines = Vec::with_capacity(points.len());
    for pair in points.windows(2) {
        lines.push((pair[0], pair[1], color));
    }
    lines.push((*points.first().unwrap(), *points.last().unwrap(), color));
    lines.into()
}

pub fn rocket() -> Box<[Line]> {
    let color = Point3::new(1.0, 0.0, 0.0);
    let vertices = [
        Point2::new(-5.0, 0.0),
        Point2::new(-10.0, 5.0),
        Point2::new(10.0, 0.0),
        Point2::new(-10.0, -5.0),
    ];
    cannonicalize(&vertices, color)
}

pub fn bullet() -> Box<[Line]> {
    let color = Point3::new(0.0, 1.0, 1.0);
    let vertices = [
        Point2::new(-2.0, 0.0),
        Point2::new(0.0, 2.0),
        Point2::new(2.0, 0.0),
        Point2::new(0.0, -2.0),
    ];
    cannonicalize(&vertices, color)
}

pub fn rectangle(b: &Box2D, color: Point3<f32>) -> Box<[Line]> {
    let vertices = [
        Point2::new(b.min.x, b.min.y),
        Point2::new(b.max.y, b.min.y),
        Point2::new(b.max.y, b.max.y),
        Point2::new(b.min.y, b.max.y),
    ];
    cannonicalize(&vertices, color)
}

pub fn circle(radius: f32, n_points: usize) -> Vec<Point2<f32>> {
    let mut points = Vec::with_capacity(n_points);
    for i in 0..n_points {
        let i = i as f32 / n_points as f32;
        let t = i * 2.0 * std::f32::consts::PI;
        points.push(Point2::new(t.cos(), t.sin()) * radius);
    }
    points
}

pub fn asteroid(radius: f32) -> Box<[Line]> {
    cannonicalize(&circle(radius, 9), Point3::new(0.7, 0.7, 0.7))
}
