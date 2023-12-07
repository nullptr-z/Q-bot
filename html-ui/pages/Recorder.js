
function recodingState() {

}

export default class Recorder {
  mediaRecorder
  recordedChunks

  async init() {
    console.log("the init of Recorder")
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    this.mediaRecorder = new MediaRecorder(stream);

    this.mediaRecorder.ondataavailable = event => {
      if (event.data.size > 0) {
        this.recordedChunks.push(event.data);
      }
    };

    this.mediaRecorder.onstop = () => {
      const audioBlob = new Blob(this.recordedChunks, { type: 'audio/mp3' });
      // const audioUrl = URL.createObjectURL(audioBlob);
      // document.getElementById('audioPlayer').src = audioUrl;

      fetch('/assistant', { method: 'POST', body: audioBlob })
    };
  }

  start() {
    this.recordedChunks = [];
    this.mediaRecorder.start()
  }

  stop() {
    this.mediaRecorder.stop();
  }
}
