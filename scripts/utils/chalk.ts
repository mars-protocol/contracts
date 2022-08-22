import chalk from 'chalk'

export const printRed = (text: unknown) => {
  console.log(chalk.red(text))
}

export const printBlue = (text: string) => {
  console.log(chalk.blue(text))
}

export const printGreen = (text: string) => {
  console.log(chalk.green(text))
}

export const printYellow = (text: string) => {
  console.log(chalk.yellow(text))
}
