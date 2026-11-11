<?php

namespace App\GameAuth\DeviceSessions;

enum DeviceSessionPurpose: string
{
    case NativeCharactersRead = 'native-characters-read';
    case NativeTicketIssue = 'native-ticket-issue';
}
