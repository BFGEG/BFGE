#!/usr/bin/env python3
import argparse
import json
import math
import statistics
from pathlib import Path


RUN_COUNT = 3
MIN_FULL_WINDOW_SECONDS = 1.0
SUMMARY_FILE = "summary.md"


def report_rows(reports, target_fps):
    rows = []
    for scenario_index, scenario in enumerate(reports[0]["config"]["scenarios"]):
        runs = [report["results"][scenario_index] for report in reports]
        if any(run["scenario"] != scenario for run in runs):
            raise ValueError("сценарии в отчётах не совпадают")
        fps_values = [run["fps"] for run in runs]
        windows = [
            window
            for run in runs
            for window in run["frame_windows"]
            if window["duration_seconds"] >= MIN_FULL_WINDOW_SECONDS
        ]
        if not windows:
            raise ValueError(f"нет полных секундных интервалов: {scenario['name']}")
        min_run_fps = min(fps_values)
        min_second_fps = min(window["fps"] for window in windows)
        rows.append(
            {
                "objects": scenario["objects"],
                "median_fps": statistics.median(fps_values),
                "min_run_fps": min_run_fps,
                "min_second_fps": min_second_fps,
                "worst_p95_ms": max(run["full_pass_ns"]["p95_ns"] for run in runs) / 1_000_000,
                "meets_target": min_run_fps >= target_fps and min_second_fps >= target_fps,
            }
        )
    return rows


def summarize(reports, target_fps):
    config = reports[0]["config"]
    gpu = reports[0]["hardware"]["gpu"]
    if any(report["config"] != config or report["hardware"]["gpu"] != gpu for report in reports):
        raise ValueError("конфигурация или GPU отличаются между запусками")

    rows = report_rows(reports, target_fps)
    if len({row["objects"] for row in rows}) != len(rows):
        raise ValueError("количество объектов должно различаться между сценариями")

    lines = [
        "# Масштабирование BFGE",
        "",
        f"GPU: {gpu}; {config['width']}×{config['height']}; VSync: {config['sync_mode']}.",
        f"Повторов: {RUN_COUNT}; прогрев: {config['warmup_seconds']} с; "
        f"замер каждого уровня: {config['scenarios'][0]['duration_seconds']} с.",
        f"Целевой FPS: {target_fps:g}.",
        "",
        "| Объектов | Медиана FPS | Худший прогон FPS | Худшая полная секунда FPS | Худший p95, мс | Цель |",
        "| ---: | ---: | ---: | ---: | ---: | :---: |",
    ]
    for row in rows:
        lines.append(
            f"| {row['objects']} | {row['median_fps']:.1f} | {row['min_run_fps']:.1f} "
            f"| {row['min_second_fps']:.1f} | {row['worst_p95_ms']:.2f} "
            f"| {'да' if row['meets_target'] else 'нет'} |"
        )

    passing = [row["objects"] for row in rows if row["meets_target"]]
    lines.extend(
        [
            "",
            "Критерий: средний FPS каждого прогона и FPS каждой полной секунды не ниже цели.",
        ]
    )
    if passing:
        lines.append(f"Максимальный проверенный уровень, достигший цели: {max(passing)} объектов.")
    else:
        lines.append("Ни один проверенный уровень не достиг цели во всех трёх прогонах.")
    lines.extend(
        [
            "Худшая полная секунда и p95 показывают просадки отдельно от среднего FPS.",
            "Это граница среди проверенных уровней, а не точное максимальное количество объектов.",
            "",
            "Исходные данные: [run-1.json](run-1.json), [run-2.json](run-2.json), "
            "[run-3.json](run-3.json).",
            "",
        ]
    )
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Сводка трёх нагрузочных прогонов BFGE")
    parser.add_argument("directory", type=Path)
    parser.add_argument("target_fps", nargs="?", type=float, default=60.0)
    args = parser.parse_args()
    if not math.isfinite(args.target_fps) or args.target_fps <= 0:
        parser.error("target_fps должен быть положительным конечным числом")

    try:
        reports = [
            json.loads((args.directory / f"run-{index}.json").read_text())
            for index in range(1, RUN_COUNT + 1)
        ]
        summary = summarize(reports, args.target_fps)
        (args.directory / SUMMARY_FILE).write_text(summary)
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.error(str(error))
    print(summary)


if __name__ == "__main__":
    main()
