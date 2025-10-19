#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
ドッキング結果レポート生成スクリプト
Report/report.ipynbから変換
"""

import glob
import os
import re
import shutil
from pathlib import Path


def main():
    """メイン実行関数"""
    print("ドッキング結果レポート生成を開始します...")

    # 現在のディレクトリを確認
    print(f"現在のディレクトリ: {os.getcwd()}")

    # ドッキング結果の解析とランキング生成
    generate_docking_ranking()

    # トップ化合物のファイルをコピー
    copy_top_compound()

    # ランキングファイルをVisualizationディレクトリにコピー
    copy_ranking_file()

    print("レポート生成が完了しました。")


def parse_smina_log(log_file):
    """
    Sminaのログファイルから mode1 の affinity 値を取得する。

    Args:
        log_file (str): ログファイルのパス

    Returns:
        float or None: 結合エネルギー値（kcal/mol）またはNone
    """
    with open(log_file, "r") as f:
        for line in f:
            if line.strip().startswith("1 "):  # mode1 の行を取得
                parts = line.split()
                if len(parts) > 1:
                    try:
                        affinity = float(parts[1])  # Affinity値（kcal/mol）
                        return affinity
                    except ValueError:
                        pass
    return None  # mode1 のデータが見つからなかった場合


def generate_docking_ranking():
    """ドッキング結果のランキングを生成"""
    print("\n=== ドッキング結果ランキング生成 ===")

    # Dockingログファイルを取得
    log_files = glob.glob("docking_results/*_log.txt")

    # 各ログから affinity を取得
    results = []
    for log_file in log_files:
        affinity = parse_smina_log(log_file)
        if affinity is not None:
            compound_name = log_file.split("/")[-1].replace("_log.txt", "")
            results.append((compound_name, affinity))

    # Affinity（結合エネルギー）の降順にソート（低いほど強い結合）
    results.sort(key=lambda x: x[1])

    # 結果をテキストファイルに出力
    output_file = "./results/docking_ranking.txt"

    with open(output_file, "w", encoding="utf-8") as f:
        f.write("ドッキング結果ランキング（結合力が強い順）\n")
        f.write("----------------------------------------\n")
        for rank, (compound, affinity) in enumerate(results, 1):
            f.write(
                f"{rank}位: 化合物 {compound}, 結合エネルギー: {affinity:.2f} kcal/mol\n"
            )

    # 結果を表示
    print(f"{len(results)}個の化合物のドッキング結果をランキングしました")
    print(f"結果は {output_file} に保存されました")

    # トップ10の結果を表示
    print("\n=== トップ10の化合物 ===")
    for rank, (compound, affinity) in enumerate(results[:10], 1):
        print(f"{rank}位: 化合物 {compound}, 結合エネルギー: {affinity:.2f} kcal/mol")


def copy_top_compound():
    """トップ化合物のファイルをコピー"""
    print("\n=== トップ化合物ファイルのコピー ===")

    # ランキングファイルを読む
    with open("./results/docking_ranking.txt", encoding="utf-8") as f:
        text = f.read()

    # 1位の化合物名を抽出
    m = re.search(r"1位:.*化合物\s+([^\s,，]+)", text)
    if not m:
        raise ValueError("1位の化合物名が見つかりません")
    top_ligand = m.group(1)  # 例: "clean_drug5371"
    print(f"Top ligand: {top_ligand}")

    # 元のSDFファイルのパス
    src = Path(f"docking_results/{top_ligand}_docked.sdf")

    # 移動先ディレクトリ（results）
    dst_dir = Path("./results")
    dst_dir.mkdir(exist_ok=True)

    # コピー先パス
    dst = dst_dir / src.name

    # ファイルをコピー
    shutil.copy(src, dst)

    print(f"{src} を {dst} にコピーしました。")


def copy_ranking_file():
    """ランキングファイルをVisualizationディレクトリにコピー"""
    print("\n=== ランキングファイルのコピー ===")

    # ランキングファイルをコピー（既にresultsに出力されているので、この処理は不要）
    print("ランキングファイルは既に ./results/ に出力されています。")


if __name__ == "__main__":
    main()
