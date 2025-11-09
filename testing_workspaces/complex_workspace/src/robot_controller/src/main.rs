// Demonstrates usage of BOTH standard ROS types and custom interface types
fn main() {
    println!("=== Robot Controller Node ===\n");

    // ============================================
    // STANDARD ROS MESSAGE TYPES
    // ============================================
    println!("--- Standard ROS Messages ---");

    // std_msgs
    let string_msg = std_msgs::msg::string::String::default();
    println!("std_msgs::String: {:?}", string_msg);

    let header = std_msgs::msg::header::Header::default();
    println!("std_msgs::Header: {:?}", header);

    let bool_msg = std_msgs::msg::bool::Bool::default();
    println!("std_msgs::Bool: {:?}", bool_msg);

    // geometry_msgs
    let point = geometry_msgs::msg::point::Point::default();
    println!("geometry_msgs::Point: {:?}", point);

    let pose = geometry_msgs::msg::pose::Pose::default();
    println!("geometry_msgs::Pose: {:?}", pose);

    let twist = geometry_msgs::msg::twist::Twist::default();
    println!("geometry_msgs::Twist: {:?}", twist);

    // sensor_msgs
    let imu = sensor_msgs::msg::imu::Imu::default();
    println!("sensor_msgs::Imu: {:?}", imu);

    let laser_scan = sensor_msgs::msg::laser_scan::LaserScan::default();
    println!("sensor_msgs::LaserScan: {:?}", laser_scan);

    // ============================================
    // CUSTOM INTERFACE TYPES
    // ============================================
    println!("\n--- Custom Interface Messages ---");

    // Custom messages
    let status = robot_interfaces::msg::robot_status::RobotStatus::default();
    println!("robot_interfaces::RobotStatus: {:?}", status);

    let reading = robot_interfaces::msg::sensor_reading::SensorReading::default();
    println!("robot_interfaces::SensorReading: {:?}", reading);

    // Custom service types
    println!("\n--- Custom Service Types ---");
    let service_req = robot_interfaces::srv::set_mode::SetModeRequest::default();
    println!("SetModeRequest: {:?}", service_req);

    let service_resp = robot_interfaces::srv::set_mode::SetModeResponse::default();
    println!("SetModeResponse: {:?}", service_resp);

    // Custom action types
    println!("\n--- Custom Action Types ---");
    let goal = robot_interfaces::action::navigate::NavigateGoal::default();
    println!("NavigateGoal: {:?}", goal);

    let result = robot_interfaces::action::navigate::NavigateResult::default();
    println!("NavigateResult: {:?}", result);

    let feedback = robot_interfaces::action::navigate::NavigateFeedback::default();
    println!("NavigateFeedback: {:?}", feedback);

    println!("\n✓ All standard and custom interfaces loaded successfully!");
}
