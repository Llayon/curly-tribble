use bevy::prelude::*;
use savage_fantasy::economy::global::GlobalResources;
use savage_fantasy::map::resources::BerryBush;
use savage_fantasy::pawn::{
    behaviors::Gathering, brain::BrainPlugin, relations::Targeting, Hunger, Settler,
};

#[test]
fn test_vertical_interaction_failure() {
    let mut app = App::new();

    // Добавляем минимально необходимые плагины и ресурсы
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(savage_fantasy::sets::SetsPlugin);
    app.init_state::<savage_fantasy::game_state::GameState>();
    app.world_mut()
        .resource_mut::<NextState<savage_fantasy::game_state::GameState>>()
        .set(savage_fantasy::game_state::GameState::Playing);
    app.update(); // Применяем смену состояния

    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.insert_resource(GlobalResources::default());

    // Добавляем систему сбора (из BrainPlugin)
    app.add_plugins(BrainPlugin);

    // 1. Спавним куст на высоте 0.0 (например, в яме или в углу)
    let bush_pos = Vec3::new(0.0, 0.0, 0.0);
    let bush_entity = app
        .world_mut()
        .spawn((
            BerryBush { food_amount: 100.0 },
            Transform::from_translation(bush_pos),
        ))
        .id();

    // 2. Спавним поселенца на высоте 2.0 (на скале рядом)
    // Горизонтальное расстояние 1.0, Вертикальное 2.0
    // 3D Дистанция = sqrt(1^2 + 2^2) = sqrt(5) = 2.23 > 1.5
    let settler_pos = Vec3::new(1.0, 2.0, 0.0);
    let settler_entity = app
        .world_mut()
        .spawn((
            Settler,
            Gathering,
            Targeting(bush_entity),
            Hunger::new(50.0),
            Transform::from_translation(settler_pos),
        ))
        .id();

    // Продвигаем время вручную чтобы FixedUpdate сработал
    {
        let mut time = app.world_mut().resource_mut::<Time<Fixed>>();
        let period = time.timestep();
        time.advance_by(period);
    }

    // Эмулируем сильный голод чтобы увидеть логи если что-то не так
    {
        let mut h = app.world_mut().get_mut::<Hunger>(settler_entity).unwrap();
        h.increase(45.0); // 50 + 45 = 95
    }

    app.world_mut().run_schedule(FixedUpdate);

    // Проверяем результат
    let hunger = app.world().get::<Hunger>(settler_entity).unwrap();

    // ВАЖНО: Мы ожидаем что голод УМЕНЬШИТСЯ (т.е. сбор сработал)
    assert!(
        hunger.value() < 95.0,
        "Hunger should DECREASE if collection works. Current: {}",
        hunger.value()
    );
}
