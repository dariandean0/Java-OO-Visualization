#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animal_hierarchy_visualization() {
        let animal_code =
            fs::read_to_string("examples/Animal.java").expect("Failed to read Animal.java");
        let dog_code = fs::read_to_string("examples/Dog.java").expect("Failed to read Dog.java");
        let trainable_code =
            fs::read_to_string("examples/Trainable.java").expect("Failed to read Trainable.java");

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
        let vehicle_code =
            fs::read_to_string("examples/Vehicle.java").expect("Failed to read Vehicle.java");

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

    #[test]
    fn test_comprehensive_java_features() {
        let complex_code = r#"
            public abstract class AbstractBase {
                protected static final String CONSTANT = "test";
                private int value;

                public AbstractBase(int value) {
                    this.value = value;
                }

                public abstract void abstractMethod();

                public final void finalMethod() {
                    System.out.println("Final method");
                }
            }

            public interface MultiInterface extends BaseInterface {
                void interfaceMethod();
                default void defaultMethod() {
                    System.out.println("Default implementation");
                }
            }

            public class ConcreteClass extends AbstractBase implements MultiInterface {
                private String name;

                public ConcreteClass(int value, String name) {
                    super(value);
                    this.name = name;
                }

                @Override
                public void abstractMethod() {
                    System.out.println("Implemented abstract method");
                }

                @Override
                public void interfaceMethod() {
                    System.out.println("Implemented interface method");
                }
            }
        "#;

        let mut visualizer = JavaVisualizer::new().unwrap();
        let result = visualizer.analyze_and_generate(complex_code).unwrap();

        // Should find all classes and interfaces
        assert!(result.analysis.classes.len() >= 2); // At least ConcreteClass and AbstractBase

        // Should have relationships
        assert!(!result.analysis.relationships.is_empty());

        // Check for abstract class
        let abstract_class = result
            .analysis
            .classes
            .iter()
            .find(|c| c.is_abstract)
            .expect("Should have abstract class");
        assert_eq!(abstract_class.name, "AbstractBase");

        // Check for interface
        let interface = result
            .analysis
            .classes
            .iter()
            .find(|c| c.is_interface)
            .expect("Should have interface");
        assert_eq!(interface.name, "MultiInterface");

        println!("Complex features result:\n{}", result.dot_code);
    }
}
