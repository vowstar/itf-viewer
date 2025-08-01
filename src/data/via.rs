// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViaConnection {
    pub name: String,
    pub from_layer: String,
    pub to_layer: String,
    pub area: f64,
    pub resistance_per_via: f64,
    pub z_position: f64,
    pub height: f64,
}

impl ViaConnection {
    pub fn new(name: String, from_layer: String, to_layer: String, area: f64, rpv: f64) -> Self {
        Self {
            name,
            from_layer,
            to_layer,
            area,
            resistance_per_via: rpv,
            z_position: 0.0,
            height: 0.0,
        }
    }

    pub fn with_geometry(mut self, z_position: f64, height: f64) -> Self {
        self.z_position = z_position;
        self.height = height;
        self
    }

    pub fn get_top_z(&self) -> f64 {
        self.z_position + self.height
    }

    pub fn get_bottom_z(&self) -> f64 {
        self.z_position
    }

    pub fn get_via_width(&self) -> f64 {
        self.area.sqrt()
    }

    pub fn get_via_height(&self) -> f64 {
        self.height
    }

    pub fn calculate_resistance(&self, num_vias: u32) -> f64 {
        if num_vias == 0 {
            f64::INFINITY
        } else {
            self.resistance_per_via / num_vias as f64
        }
    }

    pub fn connects_layers(&self, layer1: &str, layer2: &str) -> bool {
        (self.from_layer == layer1 && self.to_layer == layer2)
            || (self.from_layer == layer2 && self.to_layer == layer1)
    }

    pub fn is_contact_via(&self) -> bool {
        self.from_layer.contains("diff")
            || self.from_layer.contains("poly")
            || self.from_layer.contains("SUBSTRATE")
            || self.to_layer.contains("diff")
            || self.to_layer.contains("poly")
            || self.to_layer.contains("SUBSTRATE")
    }

    pub fn is_metal_via(&self) -> bool {
        (self.from_layer.starts_with("metal") || self.from_layer.starts_with("alpa"))
            && (self.to_layer.starts_with("metal") || self.to_layer.starts_with("alpa"))
    }

    pub fn get_via_type(&self) -> ViaType {
        if self.is_contact_via() {
            ViaType::Contact
        } else if self.is_metal_via() {
            ViaType::Metal
        } else {
            ViaType::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViaType {
    Contact,
    Metal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViaStack {
    pub vias: Vec<ViaConnection>,
    layer_to_via_map: std::collections::HashMap<String, Vec<usize>>,
}

impl ViaStack {
    pub fn new() -> Self {
        Self {
            vias: Vec::new(),
            layer_to_via_map: std::collections::HashMap::new(),
        }
    }

    pub fn add_via(&mut self, via: ViaConnection) {
        let index = self.vias.len();

        self.layer_to_via_map
            .entry(via.from_layer.clone())
            .or_default()
            .push(index);

        self.layer_to_via_map
            .entry(via.to_layer.clone())
            .or_default()
            .push(index);

        self.vias.push(via);
    }

    pub fn get_vias_for_layer(&self, layer_name: &str) -> Vec<&ViaConnection> {
        self.layer_to_via_map
            .get(layer_name)
            .map(|indices| indices.iter().filter_map(|&i| self.vias.get(i)).collect())
            .unwrap_or_default()
    }

    pub fn get_via_between_layers(&self, layer1: &str, layer2: &str) -> Option<&ViaConnection> {
        self.vias
            .iter()
            .find(|via| via.connects_layers(layer1, layer2))
    }

    pub fn get_connection_path(
        &self,
        from_layer: &str,
        to_layer: &str,
    ) -> Option<Vec<&ViaConnection>> {
        if from_layer == to_layer {
            return Some(Vec::new());
        }

        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut parent: std::collections::HashMap<String, (&ViaConnection, String)> =
            std::collections::HashMap::new();

        queue.push_back(from_layer.to_string());
        visited.insert(from_layer.to_string());

        while let Some(current_layer) = queue.pop_front() {
            if current_layer == to_layer {
                let mut path = Vec::new();
                let mut layer = to_layer.to_string();

                while let Some((via_ref, prev_layer)) = parent.get(&layer) {
                    path.push(*via_ref);
                    layer = prev_layer.clone();
                }

                path.reverse();
                return Some(path);
            }

            for via in self.get_vias_for_layer(&current_layer) {
                let next_layer = if via.from_layer == current_layer {
                    &via.to_layer
                } else {
                    &via.from_layer
                };

                if !visited.contains(next_layer) {
                    visited.insert(next_layer.clone());
                    parent.insert(next_layer.clone(), (via, current_layer.clone()));
                    queue.push_back(next_layer.clone());
                }
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        self.vias.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vias.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<ViaConnection> {
        self.vias.iter()
    }
}

impl Default for ViaStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_via_connection_creation() {
        let via = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            5.0,
        )
        .with_geometry(1.0, 0.5);

        assert_eq!(via.name, "via1");
        assert_eq!(via.from_layer, "metal1");
        assert_eq!(via.to_layer, "metal2");
        assert_eq!(via.area, 0.04);
        assert_eq!(via.resistance_per_via, 5.0);
        assert_eq!(via.z_position, 1.0);
        assert_eq!(via.height, 0.5);
        assert_eq!(via.get_top_z(), 1.5);
        assert_eq!(via.get_bottom_z(), 1.0);
    }

    #[test]
    fn test_via_width_calculation() {
        let via = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            5.0,
        );

        assert_relative_eq!(via.get_via_width(), 0.2, epsilon = 1e-10);
    }

    #[test]
    fn test_via_resistance_calculation() {
        let via = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            10.0,
        );

        assert_eq!(via.calculate_resistance(1), 10.0);
        assert_eq!(via.calculate_resistance(2), 5.0);
        assert_eq!(via.calculate_resistance(4), 2.5);
        assert_eq!(via.calculate_resistance(0), f64::INFINITY);
    }

    #[test]
    fn test_via_type_detection() {
        let contact_via = ViaConnection::new(
            "contact".to_string(),
            "pdiff".to_string(),
            "metal1".to_string(),
            0.01,
            50.0,
        );

        let metal_via = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            5.0,
        );

        assert!(contact_via.is_contact_via());
        assert!(!contact_via.is_metal_via());
        assert_eq!(contact_via.get_via_type(), ViaType::Contact);

        assert!(!metal_via.is_contact_via());
        assert!(metal_via.is_metal_via());
        assert_eq!(metal_via.get_via_type(), ViaType::Metal);
    }

    #[test]
    fn test_via_stack() {
        let mut stack = ViaStack::new();

        let via1 = ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            5.0,
        );

        let via2 = ViaConnection::new(
            "via2".to_string(),
            "metal2".to_string(),
            "metal3".to_string(),
            0.04,
            5.0,
        );

        stack.add_via(via1);
        stack.add_via(via2);

        assert_eq!(stack.len(), 2);
        assert!(!stack.is_empty());

        let vias_for_metal2 = stack.get_vias_for_layer("metal2");
        assert_eq!(vias_for_metal2.len(), 2);

        let connection = stack.get_via_between_layers("metal1", "metal2");
        assert!(connection.is_some());
        assert_eq!(connection.unwrap().name, "via1");
    }

    #[test]
    fn test_connection_path() {
        let mut stack = ViaStack::new();

        stack.add_via(ViaConnection::new(
            "via1".to_string(),
            "metal1".to_string(),
            "metal2".to_string(),
            0.04,
            5.0,
        ));

        stack.add_via(ViaConnection::new(
            "via2".to_string(),
            "metal2".to_string(),
            "metal3".to_string(),
            0.04,
            5.0,
        ));

        let path = stack.get_connection_path("metal1", "metal3");
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].name, "via1");
        assert_eq!(path[1].name, "via2");

        let no_path = stack.get_connection_path("metal1", "metal4");
        assert!(no_path.is_none());

        let same_layer = stack.get_connection_path("metal1", "metal1");
        assert!(same_layer.is_some());
        assert!(same_layer.unwrap().is_empty());
    }
}
