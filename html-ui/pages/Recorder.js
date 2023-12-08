
let recorder
let isRecording = false

function recordingState() {
  if (isRecording) {
    console.log('recordingState stop')
    recorder.stop();
  } else {
    console.log('recordingState start')
    recorder.start();
  }
  isRecording = !this.isRecording;

}

document.addEventListener("DOMContentLoaded", function () {
  console.log('initialization')
  recorder = new Recorder();
  recorder.init();
});

function startRecording() {
  console.log('startRecording')
  recorder.start()
}

class Recorder {
  mediaRecorder;
  recordedChunks = [];

  async init() {
    console.log("the init of Recorder");

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      this.mediaRecorder = new MediaRecorder(stream);

      this.mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          this.recordedChunks.push(event.data);
        }
      };

      this.mediaRecorder.onstop = () => {
        const audioBlob = new Blob(this.recordedChunks, { type: 'audio/mp3' });

        const formData = new FormData();
        formData.append('audio', audioBlob);

        fetch('/assistant',
          {
            method: 'POST',
            body: formData,
          });
      };
    } catch (error) {
      console.error('Error accessing microphone:', error.message);
    }
  }

  start() {
    this.recordedChunks = [];
    this.mediaRecorder.start();
  }

  stop() {
    this.mediaRecorder.stop();
  }
}
