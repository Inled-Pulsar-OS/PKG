import QtQuick
import QtMultimedia

Item {
    id: root
    anchors.fill: parent
    property string videoSource: ""
    property bool isPlaying: videoPlayer.playbackState === MediaPlayer.PlayingState

    MediaPlayer {
        id: videoPlayer
        source: root.videoSource
        loops: MediaPlayer.Infinite
        audioOutput: null
        videoOutput: videoOutput
        Component.onCompleted: {
            if (source != "") play()
        }
        onSourceChanged: {
            if (source != "") play()
            else stop()
        }
    }

    VideoOutput {
        id: videoOutput
        anchors.fill: parent
        fillMode: VideoOutput.PreserveAspectCrop
        visible: videoPlayer.playbackState === MediaPlayer.PlayingState
    }
}
