#[cfg(test)]
mod tests {
    use super::*;
    use crate::{graph_generator::GraphConfig, visualizer::JavaVisualizer};

    #[test]
    fn test_animal_hierarchy_visualization() {
        let animal_code = include_str!("../examples/Animal.java");
        let dog_code = include_str!("../examples/Dog.java");
        let trainable_code = include_str!("../examples/Trainable.java");

        // Combine all the code
        let combined_code = format!("{}\n\n{}\n\n{}", animal_code, trainable_code, dog_code);

        let mut visualizer = JavaVisualizer::new().unwrap();
        let result = visualizer.analyze_and_generate(&combined_code).unwrap();

        // Check that we found all three classes/interfaces
        assert_eq!(result.analysis.classes.len(), 3);

        // Check for proper relationships
        assert!(!result.analysis.relationships.is_empty());

        // Check that the dot code contains all classes
        assert!(result.dot_code.contains("Animal"));
        assert!(result.dot_code.contains("Dog"));
        assert!(result.dot_code.contains("Trainable"));

        // Check for inheritance and implementation relationships
        assert!(result.dot_code.contains("extends") || result.dot_code.contains("implements"));

        println!("Generated DOT for Animal Hierarchy:\n{}", result.dot_code);
    }

    #[test]
    fn test_vehicle_class_visualization() {
        let vehicle_code = include_str!("../examples/Vehicle.java");

        let mut visualizer = JavaVisualizer::new().unwrap();
        let result = visualizer.analyze_and_generate(&vehicle_code).unwrap();

        assert_eq!(result.analysis.classes.len(), 1);
        let vehicle_class = &result.analysis.classes[0];

        // Check class properties
        assert_eq!(vehicle_class.name, "Vehicle");
        assert_eq!(vehicle_class.visibility, "public");
        assert!(!vehicle_class.is_abstract);
        assert!(!vehicle_class.is_interface);

        // Check that we have fields and methods
        assert!(!vehicle_class.fields.is_empty());
        assert!(!vehicle_class.methods.is_empty());
        assert!(!vehicle_class.constructors.is_empty());

        // Check for static field
        let static_field = vehicle_class
            .fields
            .iter()
            .find(|f| f.is_static)
            .expect("Should have static field");
        assert_eq!(static_field.name, "totalVehicles");

        // Check for final method
        let final_method = vehicle_class
            .methods
            .iter()
            .find(|m| m.name == "getInfo")
            .expect("Should have getInfo method");

        println!("Generated DOT for Vehicle:\n{}", result.dot_code);
    }

    #[test]
    fn test_custom_graph_config() {
        let code = r#"
            public class ConfigTest {
                private String privateField;
                protected String protectedField;
                public String publicField;

                private void privateMethod() {}
                protected void protectedMethod() {}
                public void publicMethod() {}
            }
        "#;

        // Test with private members hidden
        let config = GraphConfig {
            show_private_members: false,
            show_fields: true,
            show_methods: true,
            ..Default::default()
        };

        let mut visualizer = JavaVisualizer::with_config(config).unwrap();
        let result = visualizer.analyze_and_generate(code).unwrap();

        // Should not contain private members in the dot output
        assert!(!result.dot_code.contains("privateField"));
        assert!(!result.dot_code.contains("privateMethod"));

        // Should contain public and protected members
        assert!(
            result.dot_code.contains("publicField") || result.dot_code.contains("protectedField")
        );

        // Test with fields hidden
        let config2 = GraphConfig {
            show_fields: false,
            show_methods: true,
            ..Default::default()
        };

        visualizer.update_config(config2);
        let result2 = visualizer.analyze_and_generate(code).unwrap();

        // Should not contain any fields
        assert!(!result2.dot_code.contains("Field"));

        println!("Config test result:\n{}", result2.dot_code);
    }
}
