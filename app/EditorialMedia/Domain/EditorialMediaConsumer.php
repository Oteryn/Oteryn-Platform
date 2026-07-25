<?php

namespace App\EditorialMedia\Domain;

enum EditorialMediaConsumer: string
{
    case CMS = 'cms';
    case EVENTS = 'events';
    case WIKI = 'wiki';
}
