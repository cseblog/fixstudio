document.addEventListener('DOMContentLoaded', () => {
  const fixInput = document.getElementById('fixInput');
  const timelineTable = document.getElementById('timelineTable');
  const detailTable = document.getElementById('detailTable');
  const skipHeartbeats = document.getElementById('skipHeartbeats');
  const skipCommonFields = document.getElementById('skipCommonFields');

  document.getElementById('processBtn').addEventListener('click', () => {
    processFIXData(fixInput.value);
  });

  document.getElementById('clearBtn').addEventListener('click', () => {
    fixInput.value = '';
    timelineTable.innerHTML = '';
    detailTable.innerHTML = '';
  });

  document.getElementById('sampleBtn').addEventListener('click', () => {
    fixInput.value = `8=FIX.4.1 9=61 35=A 34=1 49=EXEC 52=2021105-23:24:06 56=BANZAI 98=0 108=30 10=003
8=FIX.4.1 9=49 35=D 34=2 49=BANZAI 52=2021105-23:24:37 56=EXEC 10=328`;
  });

  function processFIXData(data) {
    const messages = data.split('\n');
    timelineTable.innerHTML = '';
    messages.forEach((message, index) => {
      const fields = parseFIXMessage(message);
      const row = document.createElement('tr');
      row.addEventListener('click', () => displayDetail(fields));
      appendCell(row, fields['52']); // Time
      appendCell(row, fields['49']); // Sender
      appendCell(row, fields['56']); // Target
      appendCell(row, fields['35']); // Message
      appendCell(row, fields['11']); // Client order ID
      appendCell(row, ''); // Detail button
      timelineTable.appendChild(row);
    });
  }

  function parseFIXMessage(message) {
    const fields = {};
    const pairs = message.split(' ');
    pairs.forEach(pair => {
      const [tag, value] = pair.split('=');
      fields[tag] = value;
    });
    return fields;
  }

  function appendCell(row, text) {
    const cell = document.createElement('td');
    cell.textContent = text || '';
    row.appendChild(cell);
  }

  function displayDetail(fields) {
    detailTable.innerHTML = '';
    Object.keys(fields).forEach(tag => {
      const row = document.createElement('tr');
      appendCell(row, tag); // Tag
      appendCell(row, getTagDescription(tag)); // Tag Description
      appendCell(row, fields[tag]); // Value
      appendCell(row, ''); // Value Description
      detailTable.appendChild(row);
    });
  }

  function getTagDescription(tag) {
    const descriptions = {
      '8': 'BeginString',
      '9': 'BodyLength',
      '35': 'MsgType',
      '34': 'MsgSeqNum',
      '49': 'SenderCompID',
      '52': 'SendingTime',
      '56': 'TargetCompID',
      '98': 'EncryptMethod',
      '108': 'HeartBtInt',
      '10': 'CheckSum'
    };
    return descriptions[tag] || '';
  }
});
