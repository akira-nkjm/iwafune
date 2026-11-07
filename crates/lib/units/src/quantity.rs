use crate::{Dimension, Unit};

/// 単位付きの量メタ(値なし)。
///
/// 次元は単位から一意に決まるため、矛盾した `(dimension, unit)` を構築できないよう
/// **単位のみを保持**し、`dimension()` は `unit.dimension()` を返す（不整合を構造的に排除）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    value: f64,
    quantity: Quantity,
}

impl Quantity {
    pub fn new(unit: Unit) -> Quantity {
        Quantity { unit }
    }

    pub fn dimension(&self) -> Dimension {
        self.unit.dimension()
    }

    pub fn unit(&self) -> Unit {
        self.unit
    }
}

impl Measurement {
    pub fn new(value: f64, quantity: Quantity) -> Measurement {
        Measurement { value, quantity }
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn quantity(&self) -> Quantity {
        self.quantity
    }

    pub fn to_si(&self) -> f64 {
        self.quantity.unit().to_si(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::named_unit;

    #[test]
    fn constructs_quantity() {
        let meter = named_unit("m").unwrap();
        let quantity = Quantity::new(meter);
        assert_eq!(quantity.dimension(), meter.dimension());
        assert_eq!(quantity.unit(), meter);
    }

    #[test]
    fn measurement_converts_to_si() {
        let wh = named_unit("Wh").unwrap();
        let quantity = Quantity::new(wh);
        let measurement = Measurement::new(2.0, quantity);
        assert_eq!(measurement.to_si(), 7200.0);
    }
}
