[![CircleCI](https://circleci.com/gh/mmmpa/learning_storage_engine.svg?style=svg)](https://circleci.com/gh/mmmpa/learning_storage_engine)

# Learning storage engine

データベースに使用されているストレージエンジンの種類には大きく分けて log-structured と page-oriented の二つがある。page-oriented は多くのデータベースで採用されている B-tree が有名である。このリポジトリでは log-structured のストレージエンジンを勉強する。

---

  # log-structured (ログ構造？)

log-structured の log は追加のみ行われる連続したレコードを指す。追加されたレコードは原則として不変 immutable である。

追加していくのみなので、書き込みは常に効率的である。

# 素朴な log-structured storage

## set

value を特定するための key とセットでレコードを set する。log の原理に従い、レコード上に同 key があるかないかに関わらず、set のたびにレコードを追加する。

そのため set は常に高速である。

## get

get においてはすべてのレコードを検索し、key が一致する最後のレコードの value を該当するものとする。

get は O(n) である。レコードが増えるに従って必然的に低速になる。特に key が存在しない場合は必ずすべてレコードを読むことになるので低速である。

## delete

log-structured の特徴として delete の扱いがある。最後に見つかった状態を有効なレコードするため、「削除」を表すレコードを記録しなくてはならない。

---

# Index の導入

読み込みの効率化のために Index というメタデータが設定される場合がある。効率化のためのデータであり、レコード自体に影響は及ぼさない。

Index は書き込み時にメンテされる。そのため、Index の導入は書き込み速度の低下を招く。

すべてのレコードについて Index が自動的には導入されないのはこの速度低下のためである。

---

# Hash indexed log-structured storage

Index として in-memory Hash Map を用いる。Hash は key としてレコードと同一の key を保持し、value としてレコードの開始位置を保持する。

## set

ファイルの末尾に追加していくが、Hash に key として id を、value として追加前の末尾位置を挿入する。

## get

Hash から id に対応する開始位置を入手し、そこから一行分のデータを読み込む。対応する開始位置が存在しない場合はレコードが不在なのでそこで処理が完了できる。

## delete

Hash からは key-value を削除するだけで達成できるが、後述の復元の観点からレコードには依然として削除された状態を記録する必要がある。

## Index の復元

この実装の Index は in-memory である。再起動においては失われるため復元される必要がある。

復元はすべてのレコードを走査し、key と pointer を Hash に挿入することで行う。そのためレコードには依然として key を含める必要がある。

# log segmentation/compaction/merging

## Segmentation

読み込み時のパフォーマンスは Index で解決ができるが、膨れ上がったファイルの参照にはオーバーヘッドが伴う。そこで一定まで log のサイズが膨らんだ段階で log の segmentation が行われ、新しい segment file が作成される。

## Compaction

分割が行われても log 全体のサイズは際限なく膨らみ続ける。どのような用法であれ、log の原理からいずれはディスクをあふれることになる。

そこでさらに一定のサイズを超えた段階で compaction が行われる。

### 重複するレコードを排除する

log の原理から言うと既にある log を編集することはないので、正確には膨れた log から現在有効なレコードを取りだし、重複のない log を作成し、以後はその新しい log を参照するようにする。

## Merging

compaction された sefment file は通常小さくなるので、規定サイズ以下になるならば複数の segment file をまとめることができる。

連結はディスクシークの観点からパフォーマンスに寄与する。

---

# 素朴な Hash index の欠点

上記の Hash Index により読み込みにも十分な速度が得られた。key の数が限定的で書き込みが頻繁に行われるような目的の場合、この実装はとてもよく働く

しかしレコードが増えるにつれ、必要な RAM もまた増えていく。大規模なアプリケーションではすぐに溢れてしまう。ディスクに逃された Hash index はパフォーマンスに悪影響を及ぼすだろう。

また範囲クエリに弱いという問題もある。id = 1..10000 のレコードを取得したい場合、必ずすべての id について確認を行う必要がある。

log-structured の書き込み速度の優位性を保ちつつ、これらの欠点を補うためのデータ構造がある。

---

# Sorted String Table (SSTable)

レコードを key の順序にして保持する SSTable というフォーマットがある。(重複する key は無い)

ソート済みのレコードは範囲クエリに対応する他、Hash index 保持する key が疎でかまわないという利点がある。

1 と 11 と 21 の開始位置を知ればその間の id がどこにあるかは十分限定される上、よく知られているようにソート済みデータからの検索効率は高い。

# Log-Structured Merge-Tree (LSM-Tree)

SSTable のソート状態を保つための選択肢として、二つの順序付き木 (に類するデータ構造) を用いる LSM-Tree というデータ構造がある。

一時的に書き込みを受け入れる in-memory の小さな木 (C0) と、その内容を定期的に compaction/merging するディスク上の大きな木 (C1) で構成されている。

## ソート状態を保つために

ソート状態を保たなければならないため、log の原理上 C1 には直接書き込めない。そこでまず C0 にデータを挿入する。(通例、赤黒木などの平衡木が用いられる)

一定のデータが溜まった段階で古い C1 と C0 の間で compaction/merging 行う。双方ともに順序付きであるから merge sort の原理で効率的に行われる。

### Cassandra

Cassandra は LSM-Tree と SSTbale を用いている。SSTable と、おそらくそれに対する疎な index の組み合わせが C1 に相当する (ここらへんは不確か)。

page-oriented のデータ構造を使うとデータの連続性がディスク上では失われ、ランダムアクセスからのシーク効率の悪さからパフォーマンス劣化を招く恐れがあるからである。

Cassandra において SSTable は compaction/merging のたびに新しいディスク上のデータとなり連続性が確保される。

## set

木に key-value を挿入する。

ここで問題となるのは、segment file への書き込みが発生するまでにクラッシュすると木にしかないデータが失われるという点である。

そこで従来の log-structured と同じく、単純に追加する log に同じデータを書き込む。この log はバランス木の復元のためだけに使われる。

## get

まずバランス木を参照し、なければ segment file を探査する。

## delete

Index が疎であるため、ある key が削除済みであるかは木と segment file を調べるまでわからない。そこでディスクアクセス回数を削減するため、ブルームフィルターというデータ構造が使われる場合がある。
