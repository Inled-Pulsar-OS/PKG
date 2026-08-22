import QtQuick
import QtMultimedia

Item {
    id: root
    anchors.fill: parent
    property string videoSource: (config.background !== undefined && (config.background.toString().endsWith(".mp4") || config.background.toString().endsWith(".webm") || config.background.toString().endsWith(".mkv") || config.background.toString().endsWith(".mov")))
                                 ? config.background.toString()
                                 : "file:///var/lib/pulsar-sddm/pulsar-wallpaper.mp4"
    property bool hasVideo: videoPlayer.playbackState === MediaPlayer.PlayingState

    MediaPlayer {
        id: videoPlayer
        source: root.videoSource
        loops: MediaPlayer.Infinite
        audioOutput: null
        videoOutput: videoOutput
        Component.onCompleted: {
            if (source.toString() !== "") {
                play()
            }
        }
        onSourceChanged: {
            if (source.toString() !== "") {
                play()
            } else {
                stop()
            }
        }
        onErrorOccurred: (error, errorString) => {
            console.log("[SDDM VideoWallpaper] Fallback to static image:", errorString)
        }
    }

    VideoOutput {
        id: videoOutput
        anchors.fill: parent
        fillMode: VideoOutput.PreserveAspectCrop
        visible: videoPlayer.playbackState === MediaPlayer.PlayingState
    }
}
