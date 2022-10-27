use std::marker::PhantomData;

use crate::implementation::dds_impl::topic_impl::AnyTopicListener;
use crate::implementation::{dds_impl::topic_impl::TopicImpl, utils::shared_object::DdsWeak};
use crate::infrastructure::error::DdsResult;
use crate::infrastructure::instance::InstanceHandle;
use crate::infrastructure::status::StatusKind;
use crate::{
    domain::domain_participant::DomainParticipant,
    infrastructure::{entity::StatusCondition, qos::TopicQos, status::InconsistentTopicStatus},
    topic_definition::topic_listener::TopicListener,
};

/// TopicDescription represents the fact that both publications and subscriptions are tied to a single data-type. Its attribute
/// type_name defines a unique resulting type for the publication or the subscription and therefore creates an implicit association
/// with a TypeSupport. TopicDescription has also a name that allows it to be retrieved locally.
/// This class is an abstract class. It is the base class for Topic, ContentFilteredTopic, and MultiTopic.
pub struct Topic<Foo> {
    topic_attributes: DdsWeak<TopicImpl>,
    phantom: PhantomData<Foo>,
}

impl<Foo> Topic<Foo> {
    pub(crate) fn new(topic_attributes: DdsWeak<TopicImpl>) -> Self {
        Self {
            topic_attributes,
            phantom: PhantomData,
        }
    }
}

impl<Foo> AsRef<DdsWeak<TopicImpl>> for Topic<Foo> {
    fn as_ref(&self) -> &DdsWeak<TopicImpl> {
        &self.topic_attributes
    }
}

impl<Foo> Topic<Foo> {
    /// This method allows the application to retrieve the INCONSISTENT_TOPIC status of the Topic.
    /// Each DomainEntity has a set of relevant communication statuses. A change of status causes the corresponding Listener to be
    /// invoked and can also be monitored by means of the associated StatusCondition.
    /// The complete list of communication status, their values, and the DomainEntities they apply to is provided in 2.2.4.1,
    /// Communication Status.
    pub fn get_inconsistent_topic_status(&self) -> DdsResult<InconsistentTopicStatus> {
        self.topic_attributes
            .upgrade()?
            .get_inconsistent_topic_status()
    }

    /// This operation returns the DomainParticipant to which the Topic Description belongs.
    pub fn get_participant(&self) -> DdsResult<DomainParticipant> {
        self.topic_attributes
            .upgrade()?
            .get_participant()
            .map(|x| DomainParticipant::new(x.downgrade()))
    }

    /// The type_name used to create the TopicDescription
    pub fn get_type_name(&self) -> DdsResult<&'static str> {
        self.topic_attributes.upgrade()?.get_type_name()
    }

    /// The name used to create the TopicDescription
    pub fn get_name(&self) -> DdsResult<String> {
        self.topic_attributes.upgrade()?.get_name()
    }
}

impl<Foo> Topic<Foo>
where
    Foo: 'static,
{
    pub fn set_qos(&self, qos: Option<TopicQos>) -> DdsResult<()> {
        self.topic_attributes.upgrade()?.set_qos(qos)
    }

    pub fn get_qos(&self) -> DdsResult<TopicQos> {
        self.topic_attributes.upgrade()?.get_qos()
    }

    pub fn set_listener(
        &self,
        a_listener: Option<Box<dyn TopicListener<Foo = Foo>>>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        #[allow(clippy::redundant_closure)]
        self.topic_attributes.upgrade()?.set_listener(
            a_listener.map::<Box<dyn AnyTopicListener>, _>(|l| Box::new(l)),
            mask,
        )
    }

    pub fn get_listener(&self) -> DdsResult<Option<Box<dyn TopicListener<Foo = Foo>>>> {
        todo!()
    }

    pub fn get_statuscondition(&self) -> DdsResult<StatusCondition> {
        self.topic_attributes.upgrade()?.get_statuscondition()
    }

    pub fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        self.topic_attributes.upgrade()?.get_status_changes()
    }

    pub fn enable(&self) -> DdsResult<()> {
        self.topic_attributes.upgrade()?.enable()
    }

    pub fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        self.topic_attributes.upgrade()?.get_instance_handle()
    }
}

pub trait AnyTopic {}
