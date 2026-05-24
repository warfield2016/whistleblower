import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Dialogs 1.3
import QtQuick.Layouts 1.15

// Whistleblower — Basecamp app UI.
//
// The QML layer is deliberately thin: it gathers user input, hands JSON to the
// doc-index module via the `logos.module()` bridge, and renders the result. All
// orchestration (storage retry, broadcast dedup, anchor batching) lives in the
// doc-index-core Rust module so this UI stays trivial to swap or restyle.
//
// Wiring expectation (Basecamp runtime injects `logos`):
//   const docIndex = logos.module("doc-index")
//   docIndex.publishFileJson(JSON.stringify(req), fileBytes) -> Promise<jsonString>
//   docIndex.anchorBatchJson(JSON.stringify(req))            -> Promise<jsonString>
//   docIndex.lookupJson(JSON.stringify({cid}))               -> Promise<jsonString>

ApplicationWindow {
    id: root
    title: "Whistleblower"
    visible: true
    width: 720
    height: 560

    readonly property var docIndex: typeof logos !== "undefined" ? logos.module("doc-index") : null
    property var lastPublish: null
    property string statusText: docIndex
        ? "Ready. Select a file to publish."
        : "doc-index module unavailable — running outside Basecamp host."

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Label {
            text: "Publish a document"
            font.pointSize: 18
            font.bold: true
        }

        RowLayout {
            spacing: 8
            Layout.fillWidth: true
            Button {
                text: filePath.text === "" ? "Choose file…" : "Change file…"
                onClicked: fileDialog.open()
            }
            Label {
                id: filePath
                text: ""
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                color: filePath.text === "" ? "#888" : "#000"
            }
        }

        FileDialog {
            id: fileDialog
            title: "Select a file to publish"
            onAccepted: filePath.text = String(fileUrl).replace("file://", "")
        }

        GridLayout {
            columns: 2
            Layout.fillWidth: true
            columnSpacing: 8
            rowSpacing: 8

            Label { text: "Title:" }
            TextField { id: titleField; Layout.fillWidth: true; placeholderText: "Short, descriptive title" }

            Label { text: "Description:" }
            TextField { id: descriptionField; Layout.fillWidth: true; placeholderText: "Optional context" }

            Label { text: "Content type:" }
            TextField {
                id: contentTypeField
                Layout.fillWidth: true
                text: "application/pdf"
                placeholderText: "e.g. application/pdf"
            }

            Label { text: "Tags (comma-separated):" }
            TextField { id: tagsField; Layout.fillWidth: true; placeholderText: "leak, finance" }

            CheckBox {
                id: anchorImmediately
                Layout.columnSpan: 2
                text: "Also anchor on-chain after publishing"
                checked: false
            }
        }

        RowLayout {
            spacing: 8
            Layout.fillWidth: true
            Button {
                id: publishButton
                text: "Publish"
                enabled: filePath.text !== "" && titleField.text !== "" && docIndex !== null
                onClicked: publish()
            }
            Button {
                text: "Anchor selected on-chain"
                enabled: lastPublish !== null && docIndex !== null
                onClicked: anchorLast()
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "Lookup CID…"
                enabled: docIndex !== null
                onClicked: lookupDialog.open()
            }
        }

        TextArea {
            id: statusArea
            Layout.fillWidth: true
            Layout.fillHeight: true
            readOnly: true
            wrapMode: TextArea.Wrap
            text: statusText
            font.family: "monospace"
        }
    }

    Dialog {
        id: lookupDialog
        title: "Lookup CID"
        standardButtons: Dialog.Ok | Dialog.Cancel
        TextField {
            id: lookupCidField
            placeholderText: "zDv..."
            width: 480
        }
        onAccepted: lookup(lookupCidField.text)
    }

    function publish() {
        const tags = tagsField.text
            .split(",")
            .map(t => t.trim())
            .filter(t => t.length > 0)

        const req = {
            title: titleField.text,
            description: descriptionField.text,
            content_type: contentTypeField.text || "application/octet-stream",
            tags: tags,
            broadcast: true,
        }

        statusText = "Uploading " + filePath.text + " …"
        docIndex.publishFileJson(JSON.stringify(req), filePath.text)
            .then(jsonStr => {
                const resp = JSON.parse(jsonStr)
                if (!resp.ok) {
                    statusText = "Publish failed: " + resp.error
                    return
                }
                lastPublish = resp
                let msg = "PUBLISHED\n"
                msg += "  cid: " + resp.cid + "\n"
                msg += "  publish_id: " + resp.publish_id + "\n"
                msg += "  metadata_hash: " + resp.metadata_hash + "\n"
                msg += "  broadcast: " + resp.broadcast + "\n"
                statusText = msg

                if (anchorImmediately.checked) {
                    anchorLast()
                }
            })
            .catch(err => {
                statusText = "Publish error: " + err
            })
    }

    function anchorLast() {
        if (lastPublish === null) {
            return
        }
        const req = {
            entries: [{
                cid: lastPublish.cid,
                metadata_hash: lastPublish.metadata_hash,
            }],
        }
        statusText += "\nAnchoring on-chain…"
        docIndex.anchorBatchJson(JSON.stringify(req))
            .then(jsonStr => {
                const resp = JSON.parse(jsonStr)
                if (!resp.ok) {
                    statusText += "\nAnchor failed: " + resp.error
                    return
                }
                statusText += "\nANCHORED tx=" + resp.tx_hash
                statusText += "\n  newly anchored: " + resp.anchored_cids.length
                statusText += "\n  skipped (already on-chain): " + resp.skipped_duplicate_cids.length
            })
            .catch(err => { statusText += "\nAnchor error: " + err })
    }

    function lookup(cid) {
        statusText = "Looking up " + cid + " …"
        docIndex.lookupJson(JSON.stringify({ cid: cid }))
            .then(jsonStr => {
                const resp = JSON.parse(jsonStr)
                if (!resp.ok) {
                    statusText = "Lookup failed: " + resp.error
                    return
                }
                if (resp.entry === null || resp.entry === undefined) {
                    statusText = "CID not in registry: " + cid
                } else {
                    let m = "REGISTRY ENTRY\n"
                    m += "  cid: " + resp.entry.cid + "\n"
                    m += "  anchor_timestamp: " + resp.entry.anchor_timestamp + "\n"
                    statusText = m
                }
            })
            .catch(err => { statusText = "Lookup error: " + err })
    }
}
