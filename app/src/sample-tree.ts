// TODO: データの層ができたら、core の索引に置き換える。今は架空のツリー。

import type { Status } from "./components/status-mark"

export type SampleFile = {
  path: string
  title: string
  status?: Status
  needsHuman?: boolean
}

export const sampleFiles: SampleFile[] = [
  { path: "README.md", title: "作業机" },
  { path: "principles/prefer-structure-over-patches.md", title: "起きない形にして直す" },
  { path: "projects/recipe-app/README.md", title: "献立プランナー" },
  {
    path: "projects/recipe-app/tasks/2026-09-07-meal-planner.md",
    title: "献立プランナーを作る",
    status: "inprogress",
  },
  {
    path: "projects/recipe-app/tasks/2026-11-09-printable-menu.md",
    title: "献立を印刷できるようにする",
    status: "inprogress",
  },
  {
    path: "projects/recipe-app/tasks/2026-11-10-decide-paper-size.md",
    title: "用紙の大きさを決める",
    status: "todo",
    needsHuman: true,
  },
  {
    path: "projects/recipe-app/tasks/2026-11-11-print-preview.md",
    title: "印刷のプレビューを出す",
    status: "todo",
  },
  {
    path: "projects/recipe-app/tasks/2026-11-12-shopping-list.md",
    title: "買い物リストを作る",
    status: "backlog",
  },
  { path: "projects/recipe-app/artifacts/print-layout.md", title: "印刷のレイアウト" },
  {
    path: "projects/recipe-app/archive/tasks/2026-10-02-recipe-import.md",
    title: "レシピを取り込む",
    status: "done",
  },
  {
    path: "projects/recipe-app/archive/tasks/2026-10-05-weekly-mail.md",
    title: "献立を毎週メールで送る",
    status: "dropped",
  },
  { path: "projects/misc/README.md", title: "そのほか" },
]
