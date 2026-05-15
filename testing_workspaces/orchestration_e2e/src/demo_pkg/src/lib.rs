#![no_std]

pub mod talker {
    use nros::{
        CallbackId, CdrReader, CdrWriter, ComponentContext, ComponentResult, DeserError,
        Deserialize, EntityId, NodeId, NodeOptions, RosMessage, SerError, Serialize, TimerDuration,
    };

    pub struct Component;

    impl nros::Component for Component {
        const NAME: &'static str = "talker";

        fn register(context: &mut ComponentContext<'_>) -> ComponentResult<()> {
            let mut node =
                context.create_node(NodeId::new("node_talker"), NodeOptions::new("talker"))?;
            let _publisher =
                node.create_publisher::<StringMsg>(EntityId::new("pub_chatter"), "chatter")?;
            let _timer = node.create_timer(
                EntityId::new("timer_publish"),
                CallbackId::new("cb_timer"),
                TimerDuration::from_millis(100),
            )?;
            Ok(())
        }
    }

    pub struct StringMsg;

    impl Serialize for StringMsg {
        fn serialize(&self, _writer: &mut CdrWriter) -> Result<(), SerError> {
            Ok(())
        }
    }

    impl Deserialize for StringMsg {
        fn deserialize(_reader: &mut CdrReader) -> Result<Self, DeserError> {
            Ok(Self)
        }
    }

    impl RosMessage for StringMsg {
        const TYPE_NAME: &'static str = "std_msgs::msg::dds_::String_";
        const TYPE_HASH: &'static str = "std_msgs/String";
    }
}
