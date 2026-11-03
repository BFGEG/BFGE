# Примеры BFGE

Примеры показывают, как собрать сцену через публичный API и менять её во время
работы. Команды запускаются из корня репозитория. Для каждого окна нужны
графическая сессия и доступный адаптер; закройте окно, чтобы завершить пример.

| Пример | Что показывает | Запуск |
| --- | --- | --- |
| [`rotating_model`](rotating_model.rs) | Импорт OBJ, glTF или GLB и вращение модели. Без аргумента открывает встроенный куб. | `cargo run --locked --example rotating_model` |
| [`lighting_demo`](lighting_demo.rs) | Два неподвижных куба и меняющееся направление света. | `cargo run --locked --example lighting_demo` |
| [`showcase`](showcase.rs) | Сетка из 1000 кубов, составная фигура и перемещение камеры между сценами. | `cargo run --release --locked --example showcase` |
| [`city_flyover`](city_flyover.rs) | Облёт города из 144 зданий при изменении света. | `cargo run --release --locked --example city_flyover` |
| [`spiral_sculpture`](spiral_sculpture.rs) | Облёт спирали из 96 цветных ступеней, которая вращается вокруг центра. | `cargo run --release --locked --example spiral_sculpture` |
| [`depth_corridor`](depth_corridor.rs) | Полёт через 30 цветных арок и обратный пролёт. | `cargo run --release --locked --example depth_corridor` |
| [`object_properties`](object_properties.rs) | Исходная модель и её экземпляр: независимый цвет, сброс цвета, скрытие и показ, чтение и изменение трансформации узла. | `cargo run --locked --example object_properties` |
| [`runtime_objects`](runtime_objects.rs) | Создание и удаление экземпляров с общей геометрией, движение, вращение и масштабирование в заранее созданных узлах. | `cargo run --locked --example runtime_objects` |
| [`runtime_model_loading`](runtime_model_loading.rs) | Последовательная загрузка трёх моделей во время `run()`, удаление предыдущей и анимация текущей. | `cargo run --locked --example runtime_model_loading` |

Укажите собственную модель аргументом `rotating_model`:

```bash
cargo run --locked --example rotating_model -- path/to/model.obj
```

Для последовательной загрузки собственных моделей во время работы передайте
один или несколько путей. Каждый файл загружается один раз с интервалом в три
секунды; последняя модель остаётся в окне:

```bash
cargo run --locked --example runtime_model_loading -- path/to/first.obj path/to/second.glb
```

Пути считаются от текущего рабочего каталога. Поддерживаемые форматы импорта
описаны в [руководстве пользователя](../docs/user-guide.md).

Движение в демонстрациях задаётся самим примером. Примеры показывают поведение
движка; методика измерений описана отдельно в
[руководстве по бенчмаркам](../benchmark/README.md).
