#!/usr/bin/env sh
if [ $# -eq 0 ]
then
  echo "No arguments supplied"
  exit 1
fi
if [ $# -eq 1 ]
then
  skylighting -S pygments -f html -r $1
  exit 0
fi
skylighting -S pygments -f html -r $1 > $2
