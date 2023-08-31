use super::*;
use geo_booleanop::boolean::{BooleanOp, Float};
use geo_types::CoordFloat;

/// If offset computing fails this error is returned.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OffsetError {
    /// This error can be produced when manipulating edges.
    EdgeError(EdgeError),
}

/// Arcs around corners are made of 5 segments by default.
pub const DEFAULT_ARC_SEGMENTS: u32 = 5;

pub trait Offset<F: CoordFloat + Float> {
    fn offset(&self, distance: F) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        self.offset_with_arc_segments(distance, DEFAULT_ARC_SEGMENTS)
    }

    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError>;
}

impl<F: EnrichedFloat> Offset<F> for geo_types::GeometryCollection<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        let mut geometry_collection_with_offset = geo_types::MultiPolygon(Vec::new());
        for geometry in self.0.iter() {
            let geometry_with_offset = geometry.offset_with_arc_segments(distance, arc_segments)?;
            geometry_collection_with_offset =
                geometry_collection_with_offset.union(&geometry_with_offset);
        }
        Ok(geometry_collection_with_offset)
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::Geometry<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        match self {
            geo_types::Geometry::Point(point) => {
                point.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::Line(line) => {
                line.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::LineString(line_tring) => {
                line_tring.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::Triangle(triangle) => triangle
                .to_polygon()
                .offset_with_arc_segments(distance, arc_segments),
            geo_types::Geometry::Rect(rect) => rect
                .to_polygon()
                .offset_with_arc_segments(distance, arc_segments),
            geo_types::Geometry::Polygon(polygon) => {
                polygon.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::MultiPoint(multi_point) => {
                multi_point.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::MultiLineString(multi_line_string) => {
                multi_line_string.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::MultiPolygon(multi_polygon) => {
                multi_polygon.offset_with_arc_segments(distance, arc_segments)
            }
            geo_types::Geometry::GeometryCollection(geometry_collection) => {
                geometry_collection.offset_with_arc_segments(distance, arc_segments)
            }
        }
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::MultiPolygon<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        let mut polygons = geo_types::MultiPolygon(Vec::new());
        for polygon in self.0.iter() {
            let polygon_with_offset = polygon.offset_with_arc_segments(distance, arc_segments)?;
            polygons = polygons.union(&polygon_with_offset);
        }
        Ok(polygons)
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::Polygon<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        let exterior_with_offset = self
            .exterior()
            .offset_with_arc_segments(distance.abs(), arc_segments)?;
        let interiors_with_offset = geo_types::MultiLineString(self.interiors().to_vec())
            .offset_with_arc_segments(distance.abs(), arc_segments)?;

        Ok(if distance.is_sign_positive() {
            self.union(&exterior_with_offset)
                .union(&interiors_with_offset)
        } else {
            self.difference(&exterior_with_offset)
                .difference(&interiors_with_offset)
        })
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::MultiLineString<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        if distance < F::zero() {
            return Ok(geo_types::MultiPolygon(Vec::new()));
        }

        let mut multi_line_string_with_offset = geo_types::MultiPolygon(Vec::new());
        for line_string in self.0.iter() {
            let line_string_with_offset =
                line_string.offset_with_arc_segments(distance, arc_segments)?;
            multi_line_string_with_offset =
                multi_line_string_with_offset.union(&line_string_with_offset);
        }
        Ok(multi_line_string_with_offset)
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::LineString<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        if distance < F::zero() {
            return Ok(geo_types::MultiPolygon(Vec::new()));
        }

        let mut line_string_with_offset = geo_types::MultiPolygon(Vec::new());
        for line in self.lines() {
            let line_with_offset = line.offset_with_arc_segments(distance, arc_segments)?;
            line_string_with_offset = line_string_with_offset.union(&line_with_offset);
        }

        let line_string_with_offset = line_string_with_offset.0.iter().skip(1).fold(
            geo_types::MultiPolygon(
                line_string_with_offset
                    .0
                    .get(0)
                    .map(|polygon| vec![polygon.clone()])
                    .unwrap_or_default(),
            ),
            |result, hole| result.difference(hole),
        );

        Ok(line_string_with_offset)
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::Line<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        if distance < F::zero() {
            return Ok(geo_types::MultiPolygon(Vec::new()));
        }

        let v1 = &self.start;
        let v2 = &self.end;
        let e1 = Edge::new(v1, v2);

        if let (Ok(in_normal), Ok(out_normal)) = (e1.inwards_normal(), e1.outwards_normal()) {
            let offsets = [
                e1.with_offset(in_normal.x * distance, in_normal.y * distance),
                e1.inverse_with_offset(out_normal.x * distance, out_normal.y * distance),
            ];

            let len = 2;
            let mut vertices = Vec::new();

            for i in 0..len {
                let current_edge = offsets.get(i).unwrap();
                let prev_edge = offsets.get((i + len + 1) % len).unwrap();
                F::create_arc(
                    &mut vertices,
                    if i == 0 { v1 } else { v2 },
                    distance,
                    &prev_edge.next,
                    &current_edge.current,
                    arc_segments,
                    true,
                );
            }

            Ok(geo_types::MultiPolygon(vec![geo_types::Polygon::new(
                geo_types::LineString(vertices),
                vec![],
            )]))
        } else {
            geo_types::Point::from(self.start).offset_with_arc_segments(distance, arc_segments)
        }
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::MultiPoint<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        if distance < F::zero() {
            return Ok(geo_types::MultiPolygon(Vec::new()));
        }

        let mut multi_point_with_offset = geo_types::MultiPolygon(Vec::new());
        for point in self.0.iter() {
            let point_with_offset = point.offset_with_arc_segments(distance, arc_segments)?;
            multi_point_with_offset = multi_point_with_offset.union(&point_with_offset);
        }
        Ok(multi_point_with_offset)
    }
}

impl<F: EnrichedFloat> Offset<F> for geo_types::Point<F> {
    fn offset_with_arc_segments(
        &self,
        distance: F,
        arc_segments: u32,
    ) -> Result<geo_types::MultiPolygon<F>, OffsetError> {
        if distance < F::zero() {
            return Ok(geo_types::MultiPolygon(Vec::new()));
        }

        let mut angle = F::zero();

        let vertice_count = match arc_segments * 2 {
            count if count % 2 == 0 => count + 1,
            count => count,
        };

        let contour = (0..vertice_count)
            .map(|_| {
                angle = angle + F::circle_frag(vertice_count); // counter-clockwise

                geo_types::Coord::from((
                    self.x() + (distance * angle.cos()),
                    self.y() + (distance * angle.sin()),
                ))
            })
            .collect();

        Ok(geo_types::MultiPolygon(vec![geo_types::Polygon::new(
            contour,
            Vec::new(),
        )]))
    }
}

trait EnrichedFloat: CoordFloat + Float {
    fn create_arc(
        vertices: &mut Vec<geo_types::Coord<Self>>,
        center: &geo_types::Coord<Self>,
        radius: Self,
        start_vertex: &geo_types::Coord<Self>,
        end_vertex: &geo_types::Coord<Self>,
        segment_count: u32,
        outwards: bool,
    );

    fn circle_frag(frags: u32) -> Self;
}

impl EnrichedFloat for f32 {
    fn create_arc(
        vertices: &mut Vec<geo_types::Coord<f32>>,
        center: &geo_types::Coord<f32>,
        radius: f32,
        start_vertex: &geo_types::Coord<f32>,
        end_vertex: &geo_types::Coord<f32>,
        segment_count: u32,
        outwards: bool,
    ) {
        let pi2 = std::f32::consts::PI * 2.0;

        let start_angle = (start_vertex.y - center.y).atan2(start_vertex.x - center.x);
        let start_angle = if start_angle.is_sign_negative() {
            start_angle + pi2
        } else {
            start_angle
        };

        let end_angle = (end_vertex.y - center.y).atan2(end_vertex.x - center.x);
        let end_angle = if end_angle.is_sign_negative() {
            end_angle + pi2
        } else {
            end_angle
        };

        let segment_count = if segment_count % 2 == 0 {
            segment_count - 1
        } else {
            segment_count
        };

        let angle = if start_angle > end_angle {
            start_angle - end_angle
        } else {
            start_angle + pi2 - end_angle
        };

        let segment_angle = if outwards { -angle } else { pi2 - angle } / segment_count as f32;

        vertices.push(*start_vertex);
        for i in 1..segment_count {
            let angle = start_angle + segment_angle * i as f32;
            vertices.push(geo_types::Coord::from((
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )));
        }
        vertices.push(*end_vertex);
    }

    fn circle_frag(frags: u32) -> Self {
        2.0 * std::f32::consts::PI / frags as f32
    }
}

impl EnrichedFloat for f64 {
    fn create_arc(
        vertices: &mut Vec<geo_types::Coord<f64>>,
        center: &geo_types::Coord<f64>,
        radius: f64,
        start_vertex: &geo_types::Coord<f64>,
        end_vertex: &geo_types::Coord<f64>,
        segment_count: u32,
        outwards: bool,
    ) {
        let pi2 = std::f64::consts::PI * 2.0;

        let start_angle = (start_vertex.y - center.y).atan2(start_vertex.x - center.x);
        let start_angle = if start_angle.is_sign_negative() {
            start_angle + pi2
        } else {
            start_angle
        };

        let end_angle = (end_vertex.y - center.y).atan2(end_vertex.x - center.x);
        let end_angle = if end_angle.is_sign_negative() {
            end_angle + pi2
        } else {
            end_angle
        };

        let segment_count = if segment_count % 2 == 0 {
            segment_count - 1
        } else {
            segment_count
        };

        let angle = if start_angle > end_angle {
            start_angle - end_angle
        } else {
            start_angle + pi2 - end_angle
        };

        let segment_angle = if outwards { -angle } else { pi2 - angle } / segment_count as f64;

        vertices.push(*start_vertex);
        for i in 1..segment_count {
            let angle = start_angle + segment_angle * i as f64;
            vertices.push(geo_types::Coord::from((
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )));
        }
        vertices.push(*end_vertex);
    }

    fn circle_frag(frags: u32) -> Self {
        2.0 * std::f64::consts::PI / f64::from(frags)
    }
}
