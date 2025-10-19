#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
In Silicoスクリーニングスクリプト
Screening/In_silico_Screening.ipynbから変換
"""

import glob
import os
import subprocess


def main():
    """メイン実行関数"""
    print("In Silicoスクリーニングを開始します...")

    # 現在のディレクトリを確認
    print(f"現在のディレクトリ: {os.getcwd()}")

    # sminaの確認
    check_smina()

    # ドッキング結果ディレクトリの作成
    create_docking_results_directory()

    # ドッキング実行
    run_docking_screening()

    print("In Silicoスクリーニングが完了しました。")


def check_smina():
    """sminaコマンドの確認"""
    print("\n=== sminaコマンドの確認 ===")

    # sminaのヘルプを表示（確認用）
    smina_path = "smina"  # システムPATHから実行

    try:
        result = subprocess.run(
            [smina_path, "--help"], capture_output=True, text=True, timeout=10
        )
        print("sminaコマンドが利用可能です。")
        print("ヘルプの最初の部分:")
        print(
            result.stdout[:500] + "..." if len(result.stdout) > 500 else result.stdout
        )
    except subprocess.TimeoutExpired:
        print("sminaコマンドの確認がタイムアウトしました。")
    except FileNotFoundError:
        print(f"エラー: {smina_path} が見つかりません。")
        print("sminaが正しくインストールされているか確認してください。")
    except Exception as e:
        print(f"sminaコマンドの確認中にエラーが発生しました: {e}")


def create_docking_results_directory():
    """ドッキング結果ディレクトリの作成"""
    print("\n=== ドッキング結果ディレクトリの作成 ===")

    os.makedirs("docking_results", exist_ok=True)
    print("docking_resultsディレクトリを作成しました。")


def run_docking_screening():
    """ドッキングスクリーニングの実行"""
    print("\n=== ドッキングスクリーニング実行 ===")

    # get only 11 ligands for test run
    ligands = glob.glob("./constructed_library/clean_drug108*.sdf")
    # get all the ligands
    # ligands = glob.glob("./constructed_library/clean_drug*.sdf")
    print(f"発見されたリガンドファイル数: {len(ligands)}")

    # sminaのパス
    smina_path = "smina"  # システムPATHから実行

    # 各リガンドに対してドッキングを実行
    for i, lig in enumerate(ligands, 1):
        fname = os.path.splitext(os.path.basename(lig))[0]
        out_sdf = f"docking_results/{fname}_docked.sdf"
        out_log = f"docking_results/{fname}_log.txt"

        print(f"\n[{i}/{len(ligands)}] Docking {fname}...")

        try:
            # sminaコマンドの実行
            result = subprocess.run(
                [
                    smina_path,
                    "-r",
                    "./5Y7J_AB_chains_fixed.pdb",
                    "-l",
                    lig,
                    "--config",
                    "config.txt",
                    "-o",
                    out_sdf,
                    "--log",
                    out_log,
                    "--scoring",
                    "vina",
                ],
                check=True,
                capture_output=True,
                text=True,
                timeout=300,
            )  # 5分のタイムアウト

            print(f"✓ {fname} のドッキングが完了しました。")

            # ログファイルから結合エネルギーを取得して表示
            affinity = extract_affinity_from_log(out_log)
            if affinity is not None:
                print(f"  結合エネルギー: {affinity:.2f} kcal/mol")

        except subprocess.TimeoutExpired:
            print(f"✗ {fname} のドッキングがタイムアウトしました。")
        except subprocess.CalledProcessError as e:
            print(f"✗ {fname} のドッキングでエラーが発生しました: {e}")
            if e.stderr:
                print(f"  エラー詳細: {e.stderr[:200]}...")
        except Exception as e:
            print(f"✗ {fname} のドッキング中に予期しないエラーが発生しました: {e}")

    print("\nドッキングスクリーニングが完了しました。")
    print("結果は docking_results/ ディレクトリに保存されています。")


def extract_affinity_from_log(log_file):
    """ログファイルから結合エネルギーを抽出"""
    try:
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
    except Exception as e:
        print(f"  ログファイル読み込みエラー: {e}")
    return None


def get_docking_summary():
    """ドッキング結果のサマリーを取得"""
    print("\n=== ドッキング結果サマリー ===")

    log_files = glob.glob("docking_results/*_log.txt")
    results = []

    for log_file in log_files:
        affinity = extract_affinity_from_log(log_file)
        if affinity is not None:
            compound_name = os.path.basename(log_file).replace("_log.txt", "")
            results.append((compound_name, affinity))

    # 結合エネルギー順にソート
    results.sort(key=lambda x: x[1])

    print(f"成功したドッキング数: {len(results)}")
    print("\nトップ5の結果:")
    for i, (compound, affinity) in enumerate(results[:5], 1):
        print(f"{i}位: {compound} - {affinity:.2f} kcal/mol")

    return results


if __name__ == "__main__":
    main()

    # オプション: ドッキング結果のサマリーを表示
    # get_docking_summary()
