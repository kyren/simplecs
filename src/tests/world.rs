use anymap::AnyMap;

use component::*;
use component_scanner::*;
use dense_component::*;
use entity::*;
use sparse_component::*;
use world::*;
use world_multi_lock::*;

#[test]
fn test_world() {
    #[derive(Clone)]
    struct PositionComponent;
    #[derive(Clone)]
    struct VelocityComponent;
    #[derive(Clone)]
    struct PlayerComponent;

    impl Component for PositionComponent {
        type Storage = DenseComponentStorage<Self>;
    }

    impl Component for VelocityComponent {
        type Storage = DenseComponentStorage<Self>;
    }

    impl Component for PlayerComponent {
        type Storage = SparseComponentStorage<Self>;
    }

    struct TaggedResource(pub EntitySet);

    let mut world = World::new();

    // Set up an example world
    {
        world.register_component::<PositionComponent>();
        world.register_component::<VelocityComponent>();
        world.register_component::<PlayerComponent>();

        for _ in 0..5 {
            let mut components = AnyMap::new();
            components.insert(PositionComponent);
            components.insert(VelocityComponent);
            world.add_entity(Some(components)).unwrap();
        }

        let mut tagged = EntitySet::new();
        for _ in 0..5 {
            let mut components = AnyMap::new();
            components.insert(PositionComponent);
            components.insert(VelocityComponent);
            let e = world.add_entity(Some(components)).unwrap();
            tagged.insert(e);
        }

        for _ in 0..5 {
            let mut components = AnyMap::new();
            components.insert(PositionComponent);
            world.add_entity(Some(components)).unwrap();
        }

        world.insert_resource(TaggedResource(tagged));

        let mut player_components = AnyMap::new();
        player_components.insert(PositionComponent);
        player_components.insert(VelocityComponent);
        player_components.insert(PlayerComponent);
        world.add_entity(Some(player_components)).unwrap();
    }

    // Run a set of queries on this world
    {
        // If we only lock multiple components / resources with multi_lock, we can be sure not to
        // deadlock due to predictable lock order, no matter the order in which they are specified.
        let (mut positions, velocities, players, tagged) = world
            .multi_lock::<(
                WriteComponent<PositionComponent>,
                ReadComponent<VelocityComponent>,
                ReadComponent<PlayerComponent>,
                ReadResource<TaggedResource>,
            )>()
            .unwrap();

        // Query entities with all of a set of components
        for (_pc, _vc) in component_scan_join((positions.scan_mut(), velocities.scan())).iter() {
            // update position for velocity, for example
        }

        // All entities with positions and velocities, so there should be 11 total
        assert_eq!(
            component_scan_join((positions.scan_mut(), velocities.scan(),))
                .iter()
                .count(),
            11
        );

        // Limit a query by another query, and there should be only one so assert so with ComponentScanner::singleton
        {
            let (_player_position, _player_velocity) =
                component_scan_join((positions.scan_mut(), velocities.scan()))
                    .limit(players.scan())
                    .singleton()
                    .unwrap();
        }

        // We can also invert queries to scan for all non-players
        for (_pc, _vc) in component_scan_join((positions.scan_mut(), velocities.scan()))
            .not(players.scan())
            .iter()
        {
            // Every position and velocity here is for a non-player
        }

        // All entities with positions and *optionally* a velocity, so 16 in total
        assert_eq!(
            component_scan_join((positions.scan_mut(), velocities.scan().opt(),))
                .iter()
                .count(),
            16
        );

        // We can limit by an EntitySet, and there are 5 tagged entities all with positions and
        // velocities, so the count of this query should be 5.
        assert_eq!(
            component_scan_join((positions.scan_mut(), velocities.scan()))
                .limit(world.scan_entity_set(&tagged.0))
                .iter()
                .count(),
            5
        );

        // We can also join any query with the full scan of entities to capture Entity out of some query
        for (_entity, _pc, _vc) in component_scan_join((
            world.scan_entities(),
            positions.scan_mut(),
            velocities.scan(),
        )).iter()
        {}
    }
}
