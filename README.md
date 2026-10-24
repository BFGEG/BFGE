# BFGE
Blazingly Fast Graphic Engine


Пример загрузки и вращения модели через публичный API:

```bash
cargo run --example rotating_model
cargo run --example rotating_model -- path/to/model.gltf
```

Закрытие окна завершает цикл и освобождает движок. Пример требует графической
сессии; обычные тесты цикла выполняются без окна и GPU.

Чтобы увидеть работу света на неподвижных кубах, запустите:

```bash
cargo run --example lighting_demo
```

Направленный свет обходит сцену примерно за восемь секунд. Грани кубов
поочерёдно светлеют и темнеют; плоскость остаётся освещённой сверху.

Сцена по умолчанию освещена фоновым и одним направленным светом. Его можно
настроить через публичный API до запуска или в обработчике кадра:

```rust
use BFGE::math::Vec3;
use BFGE::{DirectionalLight, EngineApi, SceneLighting};

engine.scene().set_lighting(SceneLighting {
    ambient: Vec3::splat(0.2),
    directional: DirectionalLight {
        direction: Vec3::new(-1.0, -1.0, -0.5),
        color: Vec3::splat(0.8),
    },
})?;
```

Направление указывает, куда идут лучи; цвет задаёт линейную RGB-интенсивность.
Нормали вычисляются из треугольников модели при загрузке на GPU. Настройки
света входят в `save_scene` и `restore_scene`.
