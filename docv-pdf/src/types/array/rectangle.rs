/// A PDF rectangle object defined by four coordinates.
///
/// In PDF, rectangles are represented as arrays of four numbers:
/// `[x1, y1, x2, y2]` where:
/// - `(x1, y1)` is the lower-left corner
/// - `(x2, y2)` is the upper-right corner
///
/// This struct normalizes the coordinates to ensure consistent
/// top-left and bottom-right points regardless of input order.
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
    /// X-coordinate of the left edge
    left: f64,
    /// Y-coordinate of the bottom edge
    bottom: f64,
    /// X-coordinate of the right edge
    right: f64,
    /// Y-coordinate of the top edge
    top: f64,
}

impl Rectangle {
    /// Creates a new rectangle from four coordinates.
    ///
    /// The coordinates are automatically normalized to ensure
    /// `left <= right` and `bottom <= top`.
    ///
    /// # Arguments
    /// * `x1` - First x-coordinate
    /// * `y1` - First y-coordinate
    /// * `x2` - Second x-coordinate
    /// * `y2` - Second y-coordinate
    ///
    /// # Returns
    /// A normalized `Rectangle` with proper left/right and bottom/top ordering.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let left = x1.min(x2);
        let right = x1.max(x2);
        let bottom = y1.min(y2);
        let top = y1.max(y2);

        Self {
            left,
            bottom,
            right,
            top,
        }
    }

    /// Returns the left edge x-coordinate.
    pub fn left(&self) -> f64 {
        self.left
    }

    /// Returns the bottom edge y-coordinate.
    pub fn bottom(&self) -> f64 {
        self.bottom
    }

    /// Returns the right edge x-coordinate.
    pub fn right(&self) -> f64 {
        self.right
    }

    /// Returns the top edge y-coordinate.
    pub fn top(&self) -> f64 {
        self.top
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }
}
