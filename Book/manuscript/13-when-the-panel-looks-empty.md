# Chapter 13 — When the Deslop panel looks empty

You open Deslop and see no results. Before assuming there are no copies, check the message in **Top Offenders**.

## Read the message first

| What you see | What to do |
|---|---|
| Starting… | Let Deslop finish opening. |
| Scanning workspace… | Wait while it checks the project. |
| Analysing changes… | Wait for it to finish checking your latest edits. |
| Filtered: … — Clear filter | Click the row to show results hidden by that filter. |
| Stopped: … | Click the message to open the Deslop log—the record of what went wrong. |
| No duplication detected | The check finished without finding duplicates in the code it checked. |

A scan in progress is not a finished result. If it stops, keep the error message so someone helping you can see the cause.

## Check whether you hid the results

A filter narrows what appears in **Top Offenders**. For example, showing only **Error** results can hide findings marked **Warning**.

Click **Clear filter** in the filtered-results row. This shows the other findings; it does not change your code.

If you see headings such as **Identical code** or folder names, click the arrow beside one to expand it. A closed heading can hide a list that is already there.

## Check the right place

Make sure VS Code has the project folder you intended to check open. Opening one file from another project does not switch the whole project Deslop is analysing.

Also, the **dedup** count at the bottom of VS Code can describe the current file. A zero there does not mean the entire project has no duplicates. Hover over it to see the explanation, then look in **Top Offenders** for the wider list.

## If something still looks wrong

If you can see copied code but Deslop does not list it, keep both file names and the relevant code. Include the message shown in the panel when asking for help.

“No duplication detected” describes the result of that check. It does not prove that every file was included or that the tool can never miss a copy.

**Try it:** open Top Offenders, read its status, clear any filter, and expand one heading. You should be able to tell whether Deslop is still checking, showing a restricted list, or reporting a finished result.
