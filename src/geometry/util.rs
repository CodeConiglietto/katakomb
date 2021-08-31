use na::*;
use std::convert::TryInto;

pub fn calculate_bresenham(p1: Point3<i32>, p2: Point3<i32>) -> Vec<Point3<i32>> {
    let mut line = Vec::new();

    let mut p = Point3::new(p1.x, p1.y, p1.z);

    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let dz = p2.z - p1.z;
    let x_inc = if dx < 0 { -1 } else { 1 };
    let l = dx.abs();
    let y_inc = if dy < 0 { -1 } else { 1 };
    let m = dy.abs();
    let z_inc = if dz < 0 { -1 } else { 1 };
    let n = dz.abs();
    let dx2 = l << 1;
    let dy2 = m << 1;
    let dz2 = n << 1;

    if l >= m && l >= n {
        let mut err_1 = dy2 - l;
        let mut err_2 = dz2 - l;
        for _i in 0..l {
            line.push(p.clone());
            if err_1 > 0 {
                p.y += y_inc;
                err_1 -= dx2;
            }
            if err_2 > 0 {
                p.z += z_inc;
                err_2 -= dx2;
            }
            err_1 += dy2;
            err_2 += dz2;
            p.x += x_inc;
        }
    } else if m >= l && m >= n {
        let mut err_1 = dx2 - m;
        let mut err_2 = dz2 - m;
        for _i in 0..m {
            line.push(p.clone());
            if err_1 > 0 {
                p.x += x_inc;
                err_1 -= dy2;
            }
            if err_2 > 0 {
                p.z += z_inc;
                err_2 -= dy2;
            }
            err_1 += dx2;
            err_2 += dz2;
            p.y += y_inc;
        }
    } else {
        let mut err_1 = dy2 - n;
        let mut err_2 = dx2 - n;
        for _i in 0..n {
            line.push(p.clone());
            if err_1 > 0 {
                p.y += y_inc;
                err_1 -= dz2;
            }
            if err_2 > 0 {
                p.x += x_inc;
                err_2 -= dz2;
            }
            err_1 += dy2;
            err_2 += dx2;
            p.z += z_inc;
        }
    }
    line.push(p.clone());

    line
}

pub fn calculate_sphere_surface(radius: usize) -> Vec<Point3<f32>> {
    let mut points = Vec::new();

    let origin = Point3::origin();

    let radius: i32 = radius.try_into().unwrap();

    for x in -radius..radius {
        for y in -radius..radius {
            for z in -radius..radius {
                let point = Point3::new(x as f32, y as f32, z as f32);

                //DISGOSDANG
                if euclidean_distance_squared(origin, point).sqrt().floor() as i32 == radius {
                    points.push(point);
                }
            }
        }
    }

    points
}

pub fn calculate_sphere(radius: i32) -> Vec<Point3<f32>> {
    let mut points = Vec::new();

    let origin = Point3::origin();

    for x in -radius..radius {
        for y in -radius..radius {
            for z in -radius..radius {
                let point = Point3::new(x as f32, y as f32, z as f32);

                //DISGOSDANG
                if euclidean_distance_squared(origin, point).sqrt().floor() as i32 <= radius {
                    points.push(point);
                }
            }
        }
    }

    points
}

pub fn euclidean_distance_squared(a: Point3<f32>, b: Point3<f32>) -> f32 {
    let x_diff = a.x - b.x;
    let y_diff = a.y - b.y;
    let z_diff = a.z - b.z;

    (x_diff * x_diff + y_diff * y_diff + z_diff * z_diff)
}
