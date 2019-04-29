[![CircleCI](https://circleci.com/gh/mmmpa/learning_storage_engine.svg?style=svg)](https://circleci.com/gh/mmmpa/learning_storage_engine)

# Learning storage engine

データベースに使用されているストレージエンジンの種類には大きく分けて log-structured と page-oriented の二つがある。page-oriented は多くのデータベースで採用されている B-tree が有名である。このリポジトリでは log-structured のストレージエンジンを勉強する。

---

  # log-structured (ログ構造？)

log-structured の log は追加のみ行われる連続したレコードを指す。追加されたレコードは原則として不変 immutable である。

追加していくのみなので、書き込みは常に効率的である。

# 素朴な log-structured storage

:link: [most_simple.rs](./src/most_simple.rs)

## set

value を特定するための key とセットでレコードを set する。log の原理に従い、レコード上に同 key があるかないかに関わらず、set のたびにレコードを追加する。

そのため set は常に高速である。

## get

get においてはすべてのレコードを検索し、key が一致する最後のレコードの value を該当するものとする。

get は O(n) である。レコードが増えるに従って必然的に低速になる。特に key が存在しない場合は必ずすべてレコードを読むことになるので低速である。

---

# Index の導入

読み込みの効率化のために Index というメタデータが設定される場合がある。効率化のためのデータであり、レコード自体に影響は及ぼさない。

Index は書き込み時にメンテされる。そのため、Index の導入は書き込み速度の低下を招く。

すべてのレコードについて Index が自動的には導入されないのはこの速度低下のためである。

---

# Hash indexed log-structured storage

:link: [hash_index.rs](./src/hash_index.rs)

Index として in-memory Hash Map を用いる。Hash は key としてレコードと同一の key を保持し、value としてレコードの開始位置を保持する。

## set

ファイルの末尾に追加していくが、Hash に key として id を、value として追加前の末尾位置を挿入する。

## get

Hash から id に対応する開始位置を入手し、そこから一行分のデータを読み込む。対応する開始位置が存在しない場合はレコードが不在なのでそこで処理が完了できる。

## Index の復元

この実装の Index は in-memory である。再起動においては失われるため復元される必要がある。

復元はすべてのレコードを走査し、key と pointer を Hash に挿入することで行う。そのためレコードには依然として key を含める必要がある。

## RAM からあふれるという問題

この Index により読み込みにも十分な速度が得られる。しかしレコードが増えるにつれ、必要な RAM もまた増えていく。大規模なアプリケーションではすぐに破綻する。

しかしレコードの量が予測可能かつ十分に小さい場合、この Index は十分に働く。
