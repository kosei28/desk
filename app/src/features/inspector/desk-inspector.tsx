import type { JSX } from "@solidjs/web"
import { For, Match, Show, Switch } from "solid-js"
import {
  Inspector,
  InspectorGroups,
  InspectorHead,
  InspectorHuman,
  InspectorItem,
  InspectorList,
  InspectorPills,
  InspectorRow,
  InspectorSection,
  InspectorStatus,
  InspectorTable,
  InspectorTag,
  InspectorUrl,
} from "../../components/inspector"
import { PathText } from "../../components/path-text"
import type { InspectorContent, InspectorFile, InspectorValue } from "./inspect"
import type { SectionMemory } from "./sections"

type DeskInspectorProps = {
  content: InspectorContent
  sections: SectionMemory
  /** ファイルの行を押したとき（新しいタブで開く。開いているならそのタブに移る） */
  onOpen: (path: string) => void
  /** URL の値を押したとき（既定のブラウザで開く） */
  onOpenUrl: (url: string) => void
}

/**
 * 右の列（[情報設計の決まり] の「右の列」）。いちばん上（パス・頭・札）の下に、
 * コード・親子と依存・関連ファイル・リンク・他からのリンクの見出しを、中身があるものだけ並べる
 */
export function DeskInspector(props: DeskInspectorProps) {
  const values = (rows: InspectorValue[]) => (
    <InspectorTable>
      <For each={rows}>
        {(row) => (
          <InspectorRow label={row.label}>
            <Switch fallback={row.value}>
              <Match when={row.form === "url"}>
                <InspectorUrl href={row.value} onOpen={() => props.onOpenUrl(row.value)} />
              </Match>
              <Match when={row.form === "path"}>
                <PathText path={row.value} />
              </Match>
            </Switch>
          </InspectorRow>
        )}
      </For>
    </InspectorTable>
  )
  const item = (file: () => InspectorFile) => (
    <InspectorItem
      title={file().title ?? file().path}
      status={file().status}
      needsHuman={file().needsHuman}
      place={file().place}
      href={file().path}
      path={file().path}
      onOpen={() => props.onOpen(file().path)}
    />
  )
  const byPath = (file: InspectorFile) => file.path
  const section = (title: string, children: () => JSX.Element) => (
    <InspectorSection
      title={title}
      open={props.sections.isOpen(title)}
      onOpenChange={(open) => props.sections.setOpen(title, open)}
    >
      {children()}
    </InspectorSection>
  )
  const hasPills = () => props.content.status !== undefined || props.content.needsHuman || props.content.tags.length > 0

  return (
    <Inspector>
      <InspectorHead path={props.content.path}>
        <Show when={props.content.head.length > 0}>{values(props.content.head)}</Show>
        <Show when={hasPills()}>
          <InspectorPills>
            <Show when={props.content.status}>{(status) => <InspectorStatus status={status()} />}</Show>
            <Show when={props.content.needsHuman}>
              <InspectorHuman />
            </Show>
            <For each={props.content.tags}>{(tag) => <InspectorTag>{tag}</InspectorTag>}</For>
          </InspectorPills>
        </Show>
      </InspectorHead>
      <Show when={props.content.code.length > 0}>{section("コード", () => values(props.content.code))}</Show>
      <Show when={props.content.family.length > 0}>
        {section("親子と依存", () => (
          <InspectorGroups>
            <For each={props.content.family} keyed={(group) => group.label}>
              {(group) => (
                <InspectorList label={group().label} each={group().files} key={byPath}>
                  {item}
                </InspectorList>
              )}
            </For>
          </InspectorGroups>
        ))}
      </Show>
      <Show when={props.content.related.length > 0}>
        {section("関連ファイル", () => (
          <InspectorGroups>
            <For each={props.content.related} keyed={(group) => group.label}>
              {(group) => (
                <InspectorList label={group().label} each={group().files} key={byPath}>
                  {item}
                </InspectorList>
              )}
            </For>
          </InspectorGroups>
        ))}
      </Show>
      <Show when={props.content.links.length > 0}>
        {section("リンク", () => <InspectorList each={props.content.links} key={byPath}>{item}</InspectorList>)}
      </Show>
      <Show when={props.content.backlinks.length > 0}>
        {section("他からのリンク", () => <InspectorList each={props.content.backlinks} key={byPath}>{item}</InspectorList>)}
      </Show>
    </Inspector>
  )
}
