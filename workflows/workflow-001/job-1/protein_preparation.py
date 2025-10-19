#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
タンパク質準備スクリプト
Protein_Preparation/Protein_Preparation.ipynbから変換
"""

import os
import subprocess
import urllib.request

import numpy as np
from Bio.PDB import PDBIO, PDBParser, Select
from openmm.app import PDBFile
from pdbfixer import PDBFixer


def main():
    """メイン実行関数"""
    print("タンパク質準備処理を開始します...")

    # 必要なライブラリのインストール確認
    print("必要なライブラリの確認中...")

    # PDBファイルのダウンロード
    pdb_file = download_pdb_file()

    # AB鎖の抽出
    output_pdb_file = extract_ab_chains(pdb_file)

    # リガンドの中心座標計算
    center_coords = calculate_ligand_center(output_pdb_file)

    # グリッドサイズ計算とconfigファイル生成
    generate_docking_config(center_coords)

    # PDBFixerによる構造修正
    fixed_pdb_file = fix_pdb_structure(output_pdb_file)

    # PDB2PQRによる電荷付与
    pqr_file = add_amber_charges(fixed_pdb_file)

    # Visualizationディレクトリにファイルをコピー
    copy_to_visualization(fixed_pdb_file)

    print("タンパク質準備処理が完了しました。")


def download_pdb_file():
    """PDBファイルをダウンロード"""
    print("\n=== PDBファイルのダウンロード ===")

    pdb_id = "5Y7J"
    pdb_file = f"./{pdb_id}.pdb"
    url = f"https://files.rcsb.org/download/{pdb_id}.pdb"

    print(f"PDB ID {pdb_id}のファイルをダウンロード中...")
    try:
        urllib.request.urlretrieve(url, pdb_file)
        print(f"ダウンロード完了: {pdb_file}")
        return pdb_file
    except Exception as e:
        print(f"ダウンロード中にエラーが発生しました: {e}")
        exit()


def extract_ab_chains(pdb_file):
    """AB鎖の存在チェックと抽出"""
    print("\n=== AB鎖の抽出 ===")

    ligand_name = "8OL"
    parser = PDBParser()
    structure = parser.get_structure("protein", pdb_file)
    chains = [chain.id for model in structure for chain in model]
    unique_chains = sorted(list(set(chains)))

    print(f"PDBファイルに存在するチェーン: {', '.join(unique_chains)}")

    # A鎖とB鎖が両方存在するかをチェック
    if "A" in unique_chains and "B" in unique_chains:
        print("A鎖とB鎖が検出されました。これらのチェーンを抽出します。")
        output_pdb_file = f"{os.path.splitext(pdb_file)[0]}_AB_chains.pdb"

        # AB鎖とリガンドを選択するクラス
        class ABChainAndLigandSelect(Select):
            def accept_chain(self, chain):
                return chain.id in ["A", "B"]

            def accept_residue(self, residue):
                return (
                    residue.get_parent().id in ["A", "B"]
                    or residue.get_resname() == ligand_name
                )

        io = PDBIO()
        io.set_structure(structure)
        io.save(output_pdb_file, ABChainAndLigandSelect())
        print(f"AB鎖とリガンドを抽出しました: {output_pdb_file}")

    else:
        print("A鎖とB鎖が検出されませんでした。元のファイルをそのまま使用します。")
        output_pdb_file = pdb_file

    return output_pdb_file


def calculate_ligand_center(output_pdb_file):
    """リガンドの結合座標（中心座標）の計算"""
    print("\n=== リガンドの中心座標計算 ===")

    ligand_name = "8OL"
    extracted_ligand_coords = []

    # 抽出または元のPDBファイルを読み込む
    parser = PDBParser()
    target_structure = parser.get_structure("target_protein", output_pdb_file)

    for model in target_structure:
        for chain in model:
            for residue in chain:
                if residue.get_resname() == ligand_name:
                    coords = [atom.get_coord() for atom in residue]
                    if coords:
                        extracted_ligand_coords.append(coords)

    # 複数のリガンドが見つかった場合、最初のもののみを使用
    if extracted_ligand_coords:
        coords_array = np.array(extracted_ligand_coords[0])
        center = np.mean(coords_array, axis=0)

        print("--- リガンドの中心座標 ---")
        print(f"リガンド名: {ligand_name}")
        print(f"中心座標 (x, y, z): {center[0]:.3f}, {center[1]:.3f}, {center[2]:.3f}")

        return center, coords_array
    else:
        print(
            f"エラー: ファイル '{output_pdb_file}' にリガンド '{ligand_name}' が見つかりませんでした。"
        )
        exit()


def generate_docking_config(center_data):
    """グリッドサイズの自動計算とconfigファイル出力"""
    print("\n=== ドッキング設定ファイル生成 ===")

    center, coords_array = center_data

    # リガンド外形からグリッドサイズを決定
    lig_min = coords_array.min(axis=0)
    lig_max = coords_array.max(axis=0)
    extent = lig_max - lig_min  # x,y,z 各方向の分子サイズ
    padding = 8.0  # 周囲に持たせる余白 [Å]
    min_size = 20.0  # 実用最小グリッド [Å]
    size_vec = np.maximum(extent + padding, min_size)

    print(
        f"推奨グリッドサイズ (Å): size_x={size_vec[0]:.1f}, size_y={size_vec[1]:.1f}, size_z={size_vec[2]:.1f}"
    )

    # コンフィグを書き出し
    config_path = "config.txt"  # 現在のディレクトリに保存
    # 現在のディレクトリに保存するため、ディレクトリ作成は不要

    config_lines = [
        f"center_x = {center[0]:.3f}",
        f"center_y = {center[1]:.3f}",
        f"center_z = {center[2]:.3f}",
        "size_x   = 15",  # {size_vec[0]:.1f}"
        "size_y   = 15",  # {size_vec[1]:.1f}"
        "size_z   = 15",  # {size_vec[2]:.1f}"
        "exhaustiveness = 8",  # 計算強度（重くして良ければ 16 などに）
        "num_modes = 5",  # 出力ポーズ数
        "energy_range = 4",  # 出力ポーズ間の最大エネルギー差
    ]

    with open(config_path, "w", encoding="utf-8") as f:
        f.write("\n".join(config_lines) + "\n")

    print(f"✔ ドッキング設定を '{config_path}' に出力しました。")

    # 参考：コマンド例
    print("\n参考コマンド例:")
    print(
        "smina --receptor protein.pdbqt --ligand ligands.sdf --config ../Screening/config.txt --out out.sdf --log docking.log"
    )
    print(
        "vina  --receptor protein.pdbqt --ligand ligand.pdbqt  --config ../Screening/config.txt --out out.pdbqt --log docking.log"
    )


def fix_pdb_structure(output_pdb_file):
    """PDBFixerによる構造の修正とクリーンアップ"""
    print(f"\n=== PDBFixerで {output_pdb_file} を処理中...")

    fixed_pdb_file = f"{os.path.splitext(output_pdb_file)[0]}_fixed.pdb"
    fixer = PDBFixer(filename=output_pdb_file)

    fixer.findMissingResidues()
    fixer.findNonstandardResidues()
    fixer.replaceNonstandardResidues()
    fixer.removeHeterogens(keepWater=False)  # 受容体のみで良いならOK
    fixer.addMissingHydrogens()

    # ← ここを fixed に直接書く（tempは使わない）
    with open(fixed_pdb_file, "w") as fout:
        PDBFile.writeFile(fixer.topology, fixer.positions, fout)
    print(f"修正後の構造を書き出しました: {fixed_pdb_file}")

    return fixed_pdb_file


def add_amber_charges(fixed_pdb_file):
    """PDB2PQRによるAMBER電荷の付与とPQRファイルの生成"""
    print(
        f"\n=== PDB2PQRで {fixed_pdb_file} にAMBER電荷を付与し、PQRファイルを生成中..."
    )

    pqr_file = f"{os.path.splitext(fixed_pdb_file)[0]}.pqr"

    try:
        subprocess.run(
            # ここを修正: --ff オプションを AMBER に設定
            ["pdb2pqr", "--ff=AMBER", fixed_pdb_file, pqr_file],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        print(f"AMBER電荷が付与されたPQRファイルを作成しました: {pqr_file}")
        return pqr_file
    except subprocess.CalledProcessError as e:
        print(f"PDB2PQRの実行中にエラーが発生しました: {e.stderr.decode()}")
        exit()
    except FileNotFoundError:
        print("エラー: pdb2pqrがシステムパスに見つかりません。")
        exit()


def copy_to_visualization(fixed_pdb_file):
    """resultsディレクトリにファイルをコピー"""
    print("\n=== resultsディレクトリへのコピー ===")

    import shutil

    # resultsディレクトリが存在しない場合は作成
    results_dir = "./results"
    os.makedirs(results_dir, exist_ok=True)

    # ファイルをコピー
    shutil.copy(fixed_pdb_file, f"{results_dir}/{os.path.basename(fixed_pdb_file)}")
    print(f"{fixed_pdb_file} を {results_dir} にコピーしました。")


if __name__ == "__main__":
    main()
